//! Snapshot helpers for the BPF-maintained tracked-PID set.
//!
//! The kernel side maintains the actual tree via `sched_process_fork`
//! and `sched_process_exit` tracepoints. This module is a thin wrapper
//! for callers that want to inspect the current set from userspace.

use libbpf_rs::MapCore as _;

use crate::skel::TracerSkel;

/// Return the set of currently-tracked PIDs from the BPF map. The
/// result is sorted ascending. The snapshot is racy with respect to
/// concurrent fork/exit events, which is acceptable for the human
/// summary; downstream emitters use the per-syscall counter map
/// instead.
pub fn snapshot(skel: &TracerSkel<'_>) -> Vec<u32> {
    let map = &skel.maps.tracked_pids;
    let mut out = Vec::new();
    for k in map.keys() {
        if k.len() == 4 {
            out.push(u32::from_ne_bytes(k.as_slice().try_into().unwrap()));
        }
    }
    out.sort_unstable();
    out
}
