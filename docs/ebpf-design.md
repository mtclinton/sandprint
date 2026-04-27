# eBPF design notes

This document covers the design choices in
[`src/bpf/tracer.bpf.c`](../src/bpf/tracer.bpf.c) and why the program
is shaped the way it is.

## Attach points

The tracer attaches to three raw tracepoints:

| Tracepoint                            | Purpose                                       |
| ------------------------------------- | --------------------------------------------- |
| `raw_tracepoint/sys_enter`            | record every syscall entry from a tracked task |
| `raw_tracepoint/sched_process_fork`   | admit children of tracked tasks               |
| `raw_tracepoint/sched_process_exit`   | evict tasks from the tracked set              |

Why raw tracepoints rather than `tp_btf` or kprobes:

- **`tp_btf`** would also work and gives you typed arguments via
  `BPF_PROG`. The raw form is one less macro and the signatures are
  small enough that we don't miss the type info.
- **kprobes** would attach to `do_syscall_64` (or its arch
  equivalent). The function's calling convention varies across
  kernel versions; the raw tracepoint signature is stable per the
  kernel's tracepoint ABI.

## CO-RE without vmlinux.h

The conventional libbpf workflow checks in a `vmlinux.h` produced by
`bpftool btf dump file /sys/kernel/btf/vmlinux format c` and includes
it in the BPF source. We avoid that for two reasons:

1. `vmlinux.h` is large (tens of MB on recent kernels) and
   architecture-specific, which complicates the cross-arch story.
2. We only need one field of one kernel struct: `task_struct.tgid`.

Instead, the BPF source declares a stub:

```c
struct task_struct {
    int tgid;
} __attribute__((preserve_access_index));
```

`preserve_access_index` tells clang to emit BTF-CO-RE relocations
for every field access. At load time, libbpf consults the running
kernel's BTF and rewrites the offsets to whatever the actual layout
is on that kernel. This is the standard CO-RE mechanism — vmlinux.h
is just a convenient way to declare every struct at once.

When the program needs more fields (e.g., to read
`task->files->fdt->fd[i]` for argument capture), the cost-benefit
flips and we'll generate vmlinux.h via `bpftool` and check it in.

## Map layout

| Map name         | Type     | Key        | Value      | Max entries |
| ---------------- | -------- | ---------- | ---------- | ----------- |
| `tracked_pids`   | HASH     | u32 (tgid) | u8 (1)     | 4096        |
| `events`         | RINGBUF  | n/a        | n/a        | 256 KiB     |
| `syscall_counts` | ARRAY    | u32 (nr)   | u64 (count)| 512         |

`tracked_pids` is a hashmap because membership is sparse. 4096
entries is generous for typical process trees; long-lived workloads
that fork heavily may need to bump this.

`syscall_counts` is an array sized for `MAX_SYSCALL_NR = 512`,
which covers x86_64 and aarch64 at the time of writing. Lookup is
a direct index, no hashing.

The ring buffer is 256 KiB, sized to comfortably hold a burst of
events between userspace polls (every 100 ms by default). On
overflow, `bpf_ringbuf_reserve` returns `NULL` and the event is
silently dropped — userspace doesn't crash, but a profile collected
during overflow will undercount. If your workload is bursty enough
to overflow the ring buffer, raise the size in `tracer.bpf.c` or
shorten the poll interval in `TracerConfig`.

## Verifier-friendly constructs

The program uses only constructs the verifier reliably accepts
across kernel versions:

- **Bounded array indexing**: every map lookup uses a key that has
  been range-checked first. `if (syscall_id < 0 || syscall_id >=
  MAX_SYSCALL_NR) return 0;` is the gate before a map lookup keyed
  by `syscall_id`.
- **No loops**: there are no loops at all in the BPF source.
- **Atomic counter updates**: `__sync_fetch_and_add(cnt, 1)` for
  the per-syscall counters; the verifier requires this for shared
  map values.
- **Reserve/submit ringbuf**: every `bpf_ringbuf_reserve` is paired
  with `bpf_ringbuf_submit`. Mismatched reserve/submit is one of
  the more common verifier rejections.

## Process tree following

`sched_process_fork` fires after the kernel has finished setting up
the child task. Args are `(struct task_struct *parent, struct
task_struct *child)`. The handler reads both `tgid`s via
`BPF_CORE_READ`, looks up the parent, and admits the child if the
parent is tracked:

```c
struct task_struct *parent = (struct task_struct *)ctx->args[0];
struct task_struct *child  = (struct task_struct *)ctx->args[1];
__u32 ppid = BPF_CORE_READ(parent, tgid);
if (!is_tracked(ppid)) return 0;
__u32 cpid = BPF_CORE_READ(child, tgid);
__u8 one = 1;
bpf_map_update_elem(&tracked_pids, &cpid, &one, BPF_ANY);
```

The userspace bookkeeping is then just a single `track_pid` call at
startup; the kernel handles the rest.

`sched_process_fork` fires for both fork (new process) and clones
that create a new thread. For threads, parent and child have the
same TGID, so `bpf_map_update_elem` is a harmless no-op (the entry
already exists). No separate code path is needed.

## Cleanup

`sched_process_exit` removes the task from `tracked_pids` so the
hashmap doesn't grow unbounded over a long-running trace. The
hashmap has a hard cap of 4096 entries, but eviction keeps the
working set small in practice.
