//! OCI runtime spec `linux.seccomp` block emitter.
//!
//! Default action denies with EPERM (errno 1) so unobserved syscalls
//! return a clean error to the application rather than killing the
//! process — easier to spot and recover from in production.

use serde::Serialize;

use super::{ProfileError, Trace};

pub fn emit(trace: &Trace) -> Result<String, ProfileError> {
    let arch_token = match trace.arch.as_str() {
        "x86_64" => "SCMP_ARCH_X86_64",
        "aarch64" => "SCMP_ARCH_AARCH64",
        other => return Err(ProfileError::UnsupportedArch(other.to_string())),
    };
    let names = trace.unique_syscall_names();
    let oci = OciSeccomp {
        default_action: "SCMP_ACT_ERRNO",
        default_errno_ret: 1,
        architectures: vec![arch_token.to_string()],
        syscalls: vec![OciSyscallRule { names, action: "SCMP_ACT_ALLOW" }],
    };
    serde_json::to_string_pretty(&oci).map_err(ProfileError::Json)
}

#[derive(Serialize)]
struct OciSeccomp {
    #[serde(rename = "defaultAction")]
    default_action: &'static str,
    #[serde(rename = "defaultErrnoRet")]
    default_errno_ret: i32,
    architectures: Vec<String>,
    syscalls: Vec<OciSyscallRule>,
}

#[derive(Serialize)]
struct OciSyscallRule {
    names: Vec<String>,
    action: &'static str,
}
