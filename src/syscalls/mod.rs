//! Syscall number → name resolution.
//!
//! Numbers differ across architectures. The initial release exposes the
//! host architecture's table; cross-arch resolution will be wired in
//! when the `--arch` CLI flag reaches the emitters.

pub mod tables;

pub use tables::{host_arch, name};
