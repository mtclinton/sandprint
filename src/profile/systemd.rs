//! systemd unit `SystemCallFilter=` directive emitter.
//!
//! systemd accepts a space-separated allowlist on a single line; an
//! empty filter is equivalent to no restriction, so callers should
//! never deploy an emitted profile produced from a zero-syscall trace.

use super::Trace;

pub fn emit(trace: &Trace) -> String {
    let names = trace.unique_syscall_names();
    let mut s = String::with_capacity(
        "SystemCallFilter=".len() + names.iter().map(|n| n.len() + 1).sum::<usize>(),
    );
    s.push_str("SystemCallFilter=");
    for (i, n) in names.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s.push_str(n);
    }
    s.push('\n');
    s
}
