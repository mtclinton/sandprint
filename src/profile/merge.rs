//! Union multiple traces into one.
//!
//! Per-event detail is dropped on merge: it doesn't compose meaningfully
//! across runs. The merged trace keeps per-syscall counts (summed) and
//! a synthetic `target_argv` recording the inputs.

use std::collections::HashMap;

use super::{ProfileError, Trace, TraceCount};

pub fn merge(traces: &[Trace]) -> Result<Trace, ProfileError> {
    if traces.is_empty() {
        return Err(ProfileError::EmptyMerge);
    }
    let arch = traces[0].arch.clone();
    let mismatched: Vec<String> = traces
        .iter()
        .filter_map(|t| {
            if t.arch != arch {
                Some(t.arch.clone())
            } else {
                None
            }
        })
        .collect();
    if !mismatched.is_empty() {
        let mut all = vec![arch];
        all.extend(mismatched);
        return Err(ProfileError::ArchMismatch(all));
    }

    let mut by_nr: HashMap<u32, (Option<String>, u64)> = HashMap::new();
    for t in traces {
        for c in &t.counts {
            let entry = by_nr
                .entry(c.syscall_nr)
                .or_insert((c.syscall_name.clone(), 0));
            if entry.0.is_none() {
                entry.0 = c.syscall_name.clone();
            }
            entry.1 += c.count;
        }
    }

    let mut counts: Vec<TraceCount> = by_nr
        .into_iter()
        .map(|(nr, (name, count))| TraceCount {
            syscall_nr: nr,
            syscall_name: name,
            count,
        })
        .collect();
    counts.sort_by_key(|c| std::cmp::Reverse(c.count));

    let target_argv = traces
        .iter()
        .map(|t| format!("[{}]", t.target_argv.join(" ")))
        .collect::<Vec<_>>()
        .join(" + ");

    Ok(Trace {
        schema_version: 1,
        arch: traces[0].arch.clone(),
        target_argv: vec![format!("merge of: {target_argv}")],
        events: vec![],
        counts,
    })
}
