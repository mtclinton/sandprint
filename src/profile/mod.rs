//! Profile data type and output emitters.
//!
//! The canonical [`Trace`] is what flows between the tracer and the
//! emitters. `profile run` and `profile attach` produce one; `profile
//! generate` consumes one and renders it as one of the formats listed
//! in [`Format`].

pub mod diff;
pub mod markdown;
pub mod merge;
pub mod oci;
pub mod seccomp_h;
pub mod systemd;
pub mod trace;

use std::path::Path;

use thiserror::Error;

pub use trace::{Trace, TraceCount, TraceEvent};

/// Output format selectable on the `profile generate` command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// The internal trace JSON (the format `profile run --output` writes).
    Json,
    /// `linux.seccomp` block of an OCI runtime spec.
    Oci,
    /// A `SystemCallFilter=` directive line for a systemd unit.
    Systemd,
    /// A C header with `#define` entries usable with libseccomp.
    SeccompH,
    /// Human-readable Markdown report.
    Markdown,
}

impl Format {
    /// Parse a CLI value into a [`Format`]. Used as a `clap` value
    /// parser so we keep clap out of this module's dependency surface.
    pub fn parse_cli(s: &str) -> Result<Self, String> {
        match s {
            "json" => Ok(Self::Json),
            "oci" => Ok(Self::Oci),
            "systemd" => Ok(Self::Systemd),
            "seccomp-h" | "seccomp_h" => Ok(Self::SeccompH),
            "markdown" | "md" => Ok(Self::Markdown),
            other => Err(format!(
                "unknown format '{other}'; expected one of: json, oci, systemd, seccomp-h, markdown"
            )),
        }
    }
}

#[derive(Error, Debug)]
pub enum ProfileError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported architecture: {0}")]
    UnsupportedArch(String),
    #[error("cannot merge zero traces")]
    EmptyMerge,
    #[error("traces have mismatched architectures: {0:?}")]
    ArchMismatch(Vec<String>),
}

/// Read a JSON trace file from disk.
pub fn load_trace<P: AsRef<Path>>(path: P) -> Result<Trace, ProfileError> {
    let f = std::fs::File::open(path.as_ref())?;
    let r = std::io::BufReader::new(f);
    serde_json::from_reader(r).map_err(ProfileError::Json)
}

/// Render a [`Trace`] to a string in the chosen [`Format`].
pub fn emit(trace: &Trace, format: Format) -> Result<String, ProfileError> {
    match format {
        Format::Json => Ok(serde_json::to_string_pretty(trace)?),
        Format::Oci => oci::emit(trace),
        Format::Systemd => Ok(systemd::emit(trace)),
        Format::SeccompH => Ok(seccomp_h::emit(trace)),
        Format::Markdown => Ok(markdown::emit(trace)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_trace(arch: &str, names: &[(u32, &str, u64)]) -> Trace {
        Trace {
            schema_version: 1,
            arch: arch.to_string(),
            target_argv: vec!["fake".into()],
            events: vec![],
            counts: names
                .iter()
                .map(|(nr, n, c)| TraceCount {
                    syscall_nr: *nr,
                    syscall_name: Some((*n).to_string()),
                    count: *c,
                })
                .collect(),
        }
    }

    #[test]
    fn unique_names_are_sorted_and_deduped() {
        let t = fake_trace(
            "x86_64",
            &[(0, "read", 5), (1, "write", 3), (3, "close", 1), (0, "read", 7)],
        );
        let names = t.unique_syscall_names();
        assert_eq!(names, vec!["close", "read", "write"]);
    }

    #[test]
    fn systemd_emit_basic() {
        let t = fake_trace("x86_64", &[(0, "read", 1), (1, "write", 1)]);
        assert_eq!(systemd::emit(&t), "SystemCallFilter=read write\n");
    }

    #[test]
    fn oci_arch_token_x86_64() {
        let t = fake_trace("x86_64", &[(0, "read", 1)]);
        let s = oci::emit(&t).unwrap();
        assert!(s.contains("SCMP_ARCH_X86_64"));
        assert!(s.contains("\"read\""));
    }

    #[test]
    fn oci_unsupported_arch_errors() {
        let t = fake_trace("riscv64", &[(0, "read", 1)]);
        assert!(matches!(oci::emit(&t), Err(ProfileError::UnsupportedArch(_))));
    }

    #[test]
    fn diff_distinguishes_sets() {
        let a = fake_trace("x86_64", &[(0, "read", 1), (1, "write", 1)]);
        let b = fake_trace("x86_64", &[(1, "write", 1), (3, "close", 1)]);
        let d = diff::diff(&a, &b);
        assert_eq!(d.only_a, vec!["read"]);
        assert_eq!(d.only_b, vec!["close"]);
        assert_eq!(d.common, vec!["write"]);
    }

    #[test]
    fn merge_sums_counts() {
        let a = fake_trace("x86_64", &[(0, "read", 5), (1, "write", 3)]);
        let b = fake_trace("x86_64", &[(0, "read", 7)]);
        let m = merge::merge(&[a, b]).unwrap();
        let read = m.counts.iter().find(|c| c.syscall_nr == 0).unwrap();
        assert_eq!(read.count, 12);
    }

    #[test]
    fn merge_rejects_arch_mismatch() {
        let a = fake_trace("x86_64", &[(0, "read", 1)]);
        let b = fake_trace("aarch64", &[(0, "read", 1)]);
        assert!(matches!(merge::merge(&[a, b]), Err(ProfileError::ArchMismatch(_))));
    }
}
