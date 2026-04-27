//! Wire layout of ring buffer events.
//!
//! The fields here must stay in lock-step with `struct syscall_event`
//! in `src/bpf/tracer.h`. The userspace consumer reinterprets ring
//! buffer payloads as [`SyscallEvent`] via `read_unaligned`.

const COMM_LEN: usize = 16;

/// One syscall entry observed by the BPF program.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SyscallEvent {
    /// Kernel monotonic timestamp in nanoseconds (`bpf_ktime_get_ns`).
    pub timestamp_ns: u64,
    /// Userspace PID (kernel `tgid`) of the calling task.
    pub tgid: u32,
    /// Userspace TID (kernel `pid`) of the calling task.
    pub tid: u32,
    /// Syscall number on the host architecture.
    pub syscall_nr: u32,
    _pad: u32,
    /// Truncated `comm` of the calling task (NUL-padded).
    pub comm: [u8; COMM_LEN],
}

impl SyscallEvent {
    /// Decode `comm` as a UTF-8 string, lossy where bytes are invalid.
    pub fn comm_str(&self) -> std::borrow::Cow<'_, str> {
        let len = self
            .comm
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.comm.len());
        String::from_utf8_lossy(&self.comm[..len])
    }
}

const _: () = assert!(std::mem::size_of::<SyscallEvent>() == 40);
