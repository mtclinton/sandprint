//! syscall-profiler observes a process via eBPF and renders a tight
//! seccomp allowlist from the syscalls it actually invokes.
//!
//! # Architecture
//!
//! The crate is organized into four user-facing modules:
//!
//! - [`tracer`] runs the eBPF program and consumes the ring buffer of
//!   syscall events.
//! - [`syscalls`] resolves syscall numbers to architecture-specific names.
//! - [`profile`] renders captured traces into one of several output
//!   formats (OCI seccomp, systemd, C header, JSON).
//! - [`cli`] is the clap-derived command-line surface.
//!
//! The [`tracer`] module is usable independently of the CLI by callers
//! that want to embed the tracer into their own tooling.
//!
//! # Privileges
//!
//! Loading the BPF program requires `CAP_BPF + CAP_PERFMON` on kernels
//! 5.8 or newer. On older kernels, `CAP_SYS_ADMIN` is required as a
//! fallback. The recommended setup for development is `setcap` rather
//! than running under sudo.

#![deny(rust_2018_idioms)]

pub mod cli;
pub mod profile;
pub mod syscalls;
pub mod tracer;

#[allow(
    clippy::all,
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_qualifications
)]
pub(crate) mod skel {
    include!(concat!(env!("OUT_DIR"), "/tracer.skel.rs"));
}

pub use tracer::{Tracer, TracerConfig, TracerError};
