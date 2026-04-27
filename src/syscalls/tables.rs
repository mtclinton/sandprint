//! Architecture-specific syscall name tables, backed by the `syscalls` crate.

use syscalls::Sysno;

/// Return the host architecture as a string slice (e.g. `"x86_64"`,
/// `"aarch64"`). Matches the values produced by `uname -m`.
pub fn host_arch() -> &'static str {
    std::env::consts::ARCH
}

/// Resolve a syscall number to its symbolic name on the host architecture.
/// Returns `None` for numbers that don't correspond to a real syscall.
pub fn name(nr: u32) -> Option<&'static str> {
    Sysno::new(nr as usize).map(|s| s.name())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn x86_64_zero_is_read() {
        assert_eq!(name(0), Some("read"));
    }

    #[test]
    fn host_arch_is_nonempty() {
        assert!(!host_arch().is_empty());
    }
}
