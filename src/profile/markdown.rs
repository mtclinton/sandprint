//! Human-readable Markdown report emitter.

use std::fmt::Write;

use super::Trace;

pub fn emit(trace: &Trace) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# sandprint profile");
    let _ = writeln!(s);
    let _ = writeln!(s, "- **Architecture:** {}", trace.arch);
    let _ = writeln!(s, "- **Command:** `{}`", trace.target_argv.join(" "));
    let _ = writeln!(s, "- **Total events:** {}", trace.events.len());
    let _ = writeln!(s, "- **Unique syscalls:** {}", trace.counts.len());
    let _ = writeln!(s);
    let _ = writeln!(s, "| NR | Count | Name |");
    let _ = writeln!(s, "|---:|------:|------|");
    for c in &trace.counts {
        let name = c.syscall_name.as_deref().unwrap_or("?");
        let _ = writeln!(s, "| {} | {} | `{}` |", c.syscall_nr, c.count, name);
    }
    s
}
