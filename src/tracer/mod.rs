//! Userspace BPF tracer: load the BPF object, attach it, expose the
//! resulting maps and ring buffer to the rest of the crate.
//!
//! The submodules split responsibilities like this:
//!
//! - [`attach`] handles the spawn-suspended fork+exec dance used by
//!   `profile run` to register a child PID before it starts executing.
//! - [`ringbuf`] defines the wire layout of ring buffer events and a
//!   thin helper for consuming them.
//! - [`pidtree`] exposes a snapshot of the currently-tracked PID set.

pub mod attach;
pub mod pidtree;
pub mod ringbuf;

use std::mem::MaybeUninit;
use std::time::Duration;

use libbpf_rs::skel::{OpenSkel, Skel, SkelBuilder};
use libbpf_rs::{MapCore as _, MapFlags, OpenObject};
use thiserror::Error;
use tracing::warn;

use crate::skel::{TracerSkel, TracerSkelBuilder};

pub use ringbuf::SyscallEvent;

const MAX_SYSCALL_NR: u32 = 512;

/// Configuration knobs for [`Tracer`].
#[derive(Debug, Clone)]
pub struct TracerConfig {
    /// If true (the default), descendants of the tracked PIDs are also
    /// traced. Implemented in BPF via `sched_process_fork`.
    pub include_children: bool,
    /// How long [`run::run`] blocks on a single ring buffer poll between
    /// child status checks. Lower values reduce shutdown latency at the
    /// cost of more `epoll_wait` syscalls in the parent.
    pub poll_interval: Duration,
}

impl Default for TracerConfig {
    fn default() -> Self {
        Self {
            include_children: true,
            poll_interval: Duration::from_millis(100),
        }
    }
}

/// Errors produced by the tracer.
#[derive(Error, Debug)]
pub enum TracerError {
    #[error("BPF error: {0}")]
    Bpf(#[from] libbpf_rs::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("nix error: {0}")]
    Nix(#[from] nix::errno::Errno),
    #[error("insufficient privileges (need CAP_BPF + CAP_PERFMON or CAP_SYS_ADMIN)")]
    Privilege,
    #[error("target command was empty")]
    EmptyCommand,
}

/// Loaded and attached BPF tracer.
pub struct Tracer<'obj> {
    skel: TracerSkel<'obj>,
    config: TracerConfig,
}

impl<'obj> Tracer<'obj> {
    /// Bump RLIMIT_MEMLOCK, open the BPF skeleton, load and attach it.
    ///
    /// `object` is uninitialized storage for the underlying `OpenObject`
    /// that backs the skeleton. The caller owns this storage; it must
    /// live at least as long as the returned [`Tracer`]. The recommended
    /// pattern is:
    ///
    /// ```ignore
    /// let mut object = std::mem::MaybeUninit::uninit();
    /// let mut tracer = Tracer::new(&mut object, TracerConfig::default())?;
    /// ```
    ///
    /// Drop order in Rust (reverse of declaration) ensures `tracer` is
    /// dropped before `object`, so the borrow stays valid for the
    /// tracer's lifetime.
    pub fn new(
        object: &'obj mut MaybeUninit<OpenObject>,
        config: TracerConfig,
    ) -> Result<Self, TracerError> {
        bump_memlock_rlimit();
        let builder = TracerSkelBuilder::default();
        let open = builder.open(object)?;
        let mut skel = open.load()?;
        skel.attach()?;
        Ok(Self { skel, config })
    }

    pub fn config(&self) -> &TracerConfig {
        &self.config
    }

    /// Add `pid` to the BPF-side tracked-PID set so that its syscalls
    /// are recorded. Children are admitted automatically by the BPF
    /// `sched_process_fork` handler when `include_children` is true.
    pub fn track_pid(&mut self, pid: u32) -> Result<(), TracerError> {
        let key = pid.to_ne_bytes();
        let val = [1u8];
        self.skel
            .maps
            .tracked_pids
            .update(&key, &val, MapFlags::ANY)?;
        Ok(())
    }

    /// Remove `pid` from the tracked-PID set.
    pub fn untrack_pid(&mut self, pid: u32) -> Result<(), TracerError> {
        let key = pid.to_ne_bytes();
        let _ = self.skel.maps.tracked_pids.delete(&key);
        Ok(())
    }

    /// Snapshot the per-syscall counter map. Returns `(syscall_nr, count)`
    /// pairs sorted by descending count, omitting syscalls with count zero.
    pub fn syscall_counts(&self) -> Result<Vec<(u32, u64)>, TracerError> {
        let mut out = Vec::new();
        let map = &self.skel.maps.syscall_counts;
        for nr in 0u32..MAX_SYSCALL_NR {
            let key = nr.to_ne_bytes();
            if let Some(v) = map.lookup(&key, MapFlags::ANY)? {
                let bytes: [u8; 8] = v.as_slice().try_into().unwrap_or([0; 8]);
                let count = u64::from_ne_bytes(bytes);
                if count > 0 {
                    out.push((nr, count));
                }
            }
        }
        out.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        Ok(out)
    }

    pub(crate) fn skel(&self) -> &TracerSkel<'obj> {
        &self.skel
    }
}

fn bump_memlock_rlimit() {
    let rlim = libc::rlimit {
        rlim_cur: libc::RLIM_INFINITY,
        rlim_max: libc::RLIM_INFINITY,
    };
    let r = unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlim) };
    if r != 0 {
        let e = std::io::Error::last_os_error();
        warn!("setrlimit(RLIMIT_MEMLOCK, INFINITY) failed: {e}; you may need CAP_SYS_RESOURCE");
    }
}
