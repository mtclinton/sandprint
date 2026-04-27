//! Spawn a child suspended on a sync pipe so we can register its PID
//! with the BPF tracker before it executes any user code.
//!
//! `std::process::Command` doesn't fit here: its `pre_exec` hook can
//! block in the child, but the parent's `spawn()` is blocked on the
//! same close-on-exec status pipe, so it can never reach the BPF map
//! before the child execs. Direct `fork(2)` is fine because we are
//! single-threaded at this point in startup.

use std::ffi::CString;
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;

use nix::fcntl::OFlag;
use nix::sys::signal::{kill, Signal};
use nix::unistd::{fork, pipe2, ForkResult, Pid};

use super::TracerError;

/// Handle to a child process that is blocked in `read(2)` on a pipe
/// inherited from the parent. The child runs no user code until
/// [`ChildHandle::release`] is called.
pub struct ChildHandle {
    pub pid: Pid,
    write_fd: Option<OwnedFd>,
}

impl ChildHandle {
    /// Wake the child so it can `execvp` the target.
    pub fn release(mut self) -> Result<(), TracerError> {
        if let Some(fd) = self.write_fd.take() {
            let r = unsafe { libc::write(fd.as_raw_fd(), b"x".as_ptr().cast(), 1) };
            if r != 1 {
                return Err(TracerError::Io(std::io::Error::last_os_error()));
            }
        }
        Ok(())
    }
}

impl Drop for ChildHandle {
    fn drop(&mut self) {
        if self.write_fd.is_some() {
            let _ = kill(self.pid, Signal::SIGKILL);
        }
    }
}

/// Fork the calling process. The parent receives a [`ChildHandle`]
/// pointing at a child that is blocked reading the sync pipe; the
/// child does not return.
pub fn spawn_suspended<S: AsRef<std::ffi::OsStr>>(args: &[S]) -> Result<ChildHandle, TracerError> {
    if args.is_empty() {
        return Err(TracerError::EmptyCommand);
    }

    let cstrings: Vec<CString> = args
        .iter()
        .map(|a| CString::new(a.as_ref().as_bytes()).expect("argv contained a NUL byte"))
        .collect();

    let (read_fd, write_fd) = pipe2(OFlag::O_CLOEXEC)?;

    // SAFETY: We are single-threaded at this point. The BPF skeleton has
    // been initialized by `Tracer::new`, but no extra threads have been
    // spawned, so it is safe to call `fork(2)` directly.
    match unsafe { fork() }? {
        ForkResult::Child => {
            drop(write_fd);
            child_main(read_fd, &cstrings);
        }
        ForkResult::Parent { child } => {
            drop(read_fd);
            Ok(ChildHandle {
                pid: child,
                write_fd: Some(write_fd),
            })
        }
    }
}

fn child_main(read_fd: OwnedFd, argv: &[CString]) -> ! {
    let mut buf = [0u8; 1];
    let r = unsafe { libc::read(read_fd.as_raw_fd(), buf.as_mut_ptr().cast(), 1) };
    if r != 1 {
        unsafe { libc::_exit(127) };
    }
    drop(read_fd);

    let mut argv_ptrs: Vec<*const libc::c_char> = argv.iter().map(|c| c.as_ptr()).collect();
    argv_ptrs.push(std::ptr::null());

    unsafe {
        let _ = libc::execvp(argv_ptrs[0], argv_ptrs.as_ptr());
    }

    let err = std::io::Error::last_os_error();
    let msg = format!("syscall-profiler: failed to exec target: {err}\n");
    unsafe {
        libc::write(libc::STDERR_FILENO, msg.as_ptr().cast(), msg.len());
        libc::_exit(127)
    }
}
