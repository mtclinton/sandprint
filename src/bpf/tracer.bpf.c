// SPDX-License-Identifier: Apache-2.0
//
// Syscall tracer.
//
// Attaches to three raw tracepoints:
//
//   - sys_enter           : every syscall entry from any task. We filter
//                           down to the tracked process tree and push an
//                           event onto the ring buffer.
//   - sched_process_fork  : when a tracked task forks, admit the child to
//                           the tracked set. This is how we follow process
//                           trees without depending on userspace bookkeeping.
//   - sched_process_exit  : evict tasks from the tracked set so the hash
//                           map doesn't fill up over long traces.
//
// The program uses CO-RE relocations to read `tgid` from `task_struct`
// without a vmlinux.h dependency. libbpf rewrites the field offset at
// load time using the running kernel's BTF.

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>

#include "tracer.h"

char LICENSE[] SEC("license") = "GPL";

/*
 * Subset of struct task_struct that we read via CO-RE.
 * preserve_access_index tells clang to emit relocations for every field
 * access; libbpf resolves them against kernel BTF at load time.
 */
struct task_struct {
    int tgid;
} __attribute__((preserve_access_index));

struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __type(key, __u32);
    __type(value, __u8);
    __uint(max_entries, 4096);
} tracked_pids SEC(".maps");

struct {
    __uint(type, BPF_MAP_TYPE_RINGBUF);
    __uint(max_entries, 1 << 18);  /* 256 KiB */
} events SEC(".maps");

struct {
    __uint(type, BPF_MAP_TYPE_ARRAY);
    __type(key, __u32);
    __type(value, __u64);
    __uint(max_entries, MAX_SYSCALL_NR);
} syscall_counts SEC(".maps");

static __always_inline int is_tracked(__u32 tgid)
{
    return bpf_map_lookup_elem(&tracked_pids, &tgid) != NULL;
}

SEC("raw_tracepoint/sys_enter")
int handle_sys_enter(struct bpf_raw_tracepoint_args *ctx)
{
    __u64 pid_tgid = bpf_get_current_pid_tgid();
    __u32 tgid = pid_tgid >> 32;
    __u32 tid = (__u32)pid_tgid;

    if (!is_tracked(tgid))
        return 0;

    long syscall_id = ctx->args[1];
    if (syscall_id < 0 || syscall_id >= MAX_SYSCALL_NR)
        return 0;

    __u32 idx = (__u32)syscall_id;
    __u64 *cnt = bpf_map_lookup_elem(&syscall_counts, &idx);
    if (cnt)
        __sync_fetch_and_add(cnt, 1);

    struct syscall_event *evt = bpf_ringbuf_reserve(&events, sizeof(*evt), 0);
    if (!evt)
        return 0;

    evt->timestamp_ns = bpf_ktime_get_ns();
    evt->tgid = tgid;
    evt->tid = tid;
    evt->syscall_nr = idx;
    evt->_pad = 0;
    bpf_get_current_comm(&evt->comm, sizeof(evt->comm));
    bpf_ringbuf_submit(evt, 0);

    return 0;
}

SEC("raw_tracepoint/sched_process_fork")
int handle_fork(struct bpf_raw_tracepoint_args *ctx)
{
    struct task_struct *parent = (struct task_struct *)ctx->args[0];
    struct task_struct *child = (struct task_struct *)ctx->args[1];

    __u32 ppid = BPF_CORE_READ(parent, tgid);
    if (!is_tracked(ppid))
        return 0;

    __u32 cpid = BPF_CORE_READ(child, tgid);
    __u8 one = 1;
    bpf_map_update_elem(&tracked_pids, &cpid, &one, BPF_ANY);

    return 0;
}

SEC("raw_tracepoint/sched_process_exit")
int handle_exit(struct bpf_raw_tracepoint_args *ctx)
{
    struct task_struct *tsk = (struct task_struct *)ctx->args[0];
    __u32 tgid = BPF_CORE_READ(tsk, tgid);
    bpf_map_delete_elem(&tracked_pids, &tgid);

    return 0;
}
