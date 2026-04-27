//! Profile emitters: convert recorded traces into formats that other
//! tools consume (OCI runtime spec, systemd unit directives, libseccomp
//! C headers, human-readable Markdown).
//!
//! The initial release focuses on the tracer pipeline, so this module
//! is intentionally small. The emitters land in follow-up commits as the
//! `profile generate` subcommand is wired up.

/// Output formats that the `profile generate` subcommand will support.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Internal trace JSON (the format `profile run --output` emits).
    Json,
    /// `linux.seccomp` block of an OCI runtime spec.
    Oci,
    /// A `SystemCallFilter=` directive line for a systemd unit.
    Systemd,
    /// A C header with `#define` entries usable with libseccomp.
    SeccompHeader,
    /// Human-readable Markdown report.
    Markdown,
}
