//! Shared output helpers used by `profile run` and `profile attach`:
//! converting the in-memory event log to the canonical [`Trace`] type
//! and printing the per-syscall summary table.

use std::io::Write;

use crate::profile::{Trace, TraceCount, TraceEvent};
use crate::syscalls;
use crate::tracer::ringbuf::SyscallEvent;

pub(crate) fn build_trace(
    argv: &[String],
    events: &[SyscallEvent],
    counts: &[(u32, u64)],
) -> Trace {
    Trace {
        schema_version: 1,
        arch: syscalls::host_arch().to_string(),
        target_argv: argv.to_vec(),
        events: events
            .iter()
            .map(|e| TraceEvent {
                timestamp_ns: e.timestamp_ns,
                tgid: e.tgid,
                tid: e.tid,
                syscall_nr: e.syscall_nr,
                syscall_name: syscalls::name(e.syscall_nr).map(String::from),
                comm: e.comm_str().into_owned(),
            })
            .collect(),
        counts: counts
            .iter()
            .map(|(nr, c)| TraceCount {
                syscall_nr: *nr,
                syscall_name: syscalls::name(*nr).map(String::from),
                count: *c,
            })
            .collect(),
    }
}

pub(crate) fn print_summary(events_len: usize, counts: &[(u32, u64)]) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(
        out,
        "Observed {} syscall events ({} unique syscalls)",
        events_len,
        counts.len()
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "{:>5}  {:>10}  NAME", "NR", "COUNT");
    for (nr, count) in counts {
        let name = syscalls::name(*nr).unwrap_or("?");
        let _ = writeln!(out, "{nr:>5}  {count:>10}  {name}");
    }
}
