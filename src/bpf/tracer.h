/* SPDX-License-Identifier: Apache-2.0 */
#ifndef __TRACER_H
#define __TRACER_H

#define MAX_SYSCALL_NR 512
#define TASK_COMM_LEN 16

/*
 * Layout shared between the BPF program and the userspace consumer.
 * Keep the field order and explicit padding here in sync with the
 * `SyscallEvent` struct in src/tracer/ringbuf.rs; the userspace side
 * `transmute`s ring buffer payloads into that struct.
 */
struct syscall_event {
    unsigned long long timestamp_ns;
    unsigned int tgid;
    unsigned int tid;
    unsigned int syscall_nr;
    unsigned int _pad;
    char comm[TASK_COMM_LEN];
};

#endif /* __TRACER_H */
