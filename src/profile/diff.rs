//! Set-difference between two traces, by syscall name.

use std::collections::BTreeSet;
use std::fmt::Write;

use super::Trace;

#[derive(Debug, Clone)]
pub struct DiffResult {
    pub only_a: Vec<String>,
    pub only_b: Vec<String>,
    pub common: Vec<String>,
}

impl DiffResult {
    pub fn render(&self, a_label: &str, b_label: &str) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "Only in {a_label} ({} syscalls):", self.only_a.len());
        for n in &self.only_a {
            let _ = writeln!(s, "  + {n}");
        }
        let _ = writeln!(s);
        let _ = writeln!(s, "Only in {b_label} ({} syscalls):", self.only_b.len());
        for n in &self.only_b {
            let _ = writeln!(s, "  - {n}");
        }
        let _ = writeln!(s);
        let _ = writeln!(s, "In both ({} syscalls)", self.common.len());
        s
    }
}

pub fn diff(a: &Trace, b: &Trace) -> DiffResult {
    let a_names: BTreeSet<String> = a
        .counts
        .iter()
        .filter_map(|c| c.syscall_name.clone())
        .collect();
    let b_names: BTreeSet<String> = b
        .counts
        .iter()
        .filter_map(|c| c.syscall_name.clone())
        .collect();

    let only_a: Vec<String> = a_names.difference(&b_names).cloned().collect();
    let only_b: Vec<String> = b_names.difference(&a_names).cloned().collect();
    let common: Vec<String> = a_names.intersection(&b_names).cloned().collect();

    DiffResult {
        only_a,
        only_b,
        common,
    }
}
