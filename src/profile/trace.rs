//! Canonical trace data type that flows from the tracer to the emitters.

use serde::{Deserialize, Serialize};

/// One full recorded session: argv, per-event log, and per-syscall counts.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Trace {
    /// Bumped whenever the on-disk JSON layout changes incompatibly.
    pub schema_version: u32,
    /// Architecture token matching `uname -m` (`x86_64`, `aarch64`).
    pub arch: String,
    /// Command line (or synthetic descriptor like `pid:1234`).
    pub target_argv: Vec<String>,
    /// Per-event log, in order of arrival on the ring buffer.
    pub events: Vec<TraceEvent>,
    /// Per-syscall counts, sorted by descending count.
    pub counts: Vec<TraceCount>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TraceEvent {
    pub timestamp_ns: u64,
    pub tgid: u32,
    pub tid: u32,
    pub syscall_nr: u32,
    pub syscall_name: Option<String>,
    pub comm: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TraceCount {
    pub syscall_nr: u32,
    pub syscall_name: Option<String>,
    pub count: u64,
}

impl Trace {
    /// Sorted, deduplicated list of syscall names observed in this trace.
    /// Entries with no resolved name (e.g., out-of-table syscall numbers)
    /// are skipped because the emitters need a name.
    pub fn unique_syscall_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .counts
            .iter()
            .filter_map(|c| c.syscall_name.clone())
            .collect();
        names.sort();
        names.dedup();
        names
    }
}
