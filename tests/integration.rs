//! End-to-end CLI tests driving the actual sandprint binary via `assert_cmd`.
//!
//! Tests that do not exercise the BPF tracer (everything except `profile
//! run` and `profile attach`) run unconditionally. The BPF-exercising
//! cases are marked `#[ignore]` and require either `CAP_BPF + CAP_PERFMON`
//! on the test runner or running as root. Run them via:
//!
//!     cargo test -- --ignored

use std::path::PathBuf;
use std::process::Command;

use assert_cmd::Command as AssertCommand;
use predicates::prelude::*;
use tempfile::tempdir;

const TRACE_A: &str = r#"{
  "schema_version": 1,
  "arch": "x86_64",
  "target_argv": ["a"],
  "events": [],
  "counts": [
    {"syscall_nr": 0, "syscall_name": "read", "count": 5},
    {"syscall_nr": 1, "syscall_name": "write", "count": 3}
  ]
}"#;

const TRACE_B: &str = r#"{
  "schema_version": 1,
  "arch": "x86_64",
  "target_argv": ["b"],
  "events": [],
  "counts": [
    {"syscall_nr": 1, "syscall_name": "write", "count": 7},
    {"syscall_nr": 257, "syscall_name": "openat", "count": 2}
  ]
}"#;

fn write_temp_trace(dir: &tempfile::TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, content).unwrap();
    path
}

#[test]
fn version_prints_crate_name() {
    AssertCommand::cargo_bin("sandprint")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("sandprint"));
}

#[test]
fn generate_systemd_format() {
    let dir = tempdir().unwrap();
    let trace = write_temp_trace(&dir, "a.json", TRACE_A);
    AssertCommand::cargo_bin("sandprint")
        .unwrap()
        .args(["profile", "generate", "--input"])
        .arg(&trace)
        .args(["--format", "systemd"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("SystemCallFilter=read write"));
}

#[test]
fn generate_oci_format() {
    let dir = tempdir().unwrap();
    let trace = write_temp_trace(&dir, "a.json", TRACE_A);
    AssertCommand::cargo_bin("sandprint")
        .unwrap()
        .args(["profile", "generate", "--input"])
        .arg(&trace)
        .args(["--format", "oci"])
        .assert()
        .success()
        .stdout(predicate::str::contains("SCMP_ARCH_X86_64"))
        .stdout(predicate::str::contains("SCMP_ACT_ALLOW"));
}

#[test]
fn generate_seccomp_h_format() {
    let dir = tempdir().unwrap();
    let trace = write_temp_trace(&dir, "a.json", TRACE_A);
    AssertCommand::cargo_bin("sandprint")
        .unwrap()
        .args(["profile", "generate", "--input"])
        .arg(&trace)
        .args(["--format", "seccomp-h"])
        .assert()
        .success()
        .stdout(predicate::str::contains("SCMP_SYS(read)"))
        .stdout(predicate::str::contains("SANDPRINT_PROFILE_H"));
}

#[test]
fn generate_markdown_format() {
    let dir = tempdir().unwrap();
    let trace = write_temp_trace(&dir, "a.json", TRACE_A);
    AssertCommand::cargo_bin("sandprint")
        .unwrap()
        .args(["profile", "generate", "--input"])
        .arg(&trace)
        .args(["--format", "markdown"])
        .assert()
        .success()
        .stdout(predicate::str::contains("# sandprint profile"))
        .stdout(predicate::str::contains("| `read` |"));
}

#[test]
fn diff_distinguishes_traces() {
    let dir = tempdir().unwrap();
    let a = write_temp_trace(&dir, "a.json", TRACE_A);
    let b = write_temp_trace(&dir, "b.json", TRACE_B);
    AssertCommand::cargo_bin("sandprint")
        .unwrap()
        .args(["profile", "diff"])
        .arg(&a)
        .arg(&b)
        .assert()
        .success()
        .stdout(predicate::str::contains("+ read"))
        .stdout(predicate::str::contains("- openat"));
}

#[test]
fn merge_sums_counts() {
    let dir = tempdir().unwrap();
    let a = write_temp_trace(&dir, "a.json", TRACE_A);
    let b = write_temp_trace(&dir, "b.json", TRACE_B);
    AssertCommand::cargo_bin("sandprint")
        .unwrap()
        .args(["profile", "merge"])
        .arg(&a)
        .arg(&b)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"count\": 10"));
}

#[test]
fn unknown_format_errors() {
    let dir = tempdir().unwrap();
    let trace = write_temp_trace(&dir, "a.json", TRACE_A);
    AssertCommand::cargo_bin("sandprint")
        .unwrap()
        .args(["profile", "generate", "--input"])
        .arg(&trace)
        .args(["--format", "bogus"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown format"));
}

#[test]
fn diff_arch_is_independent() {
    // Different archs are still diff-able (set difference is by name).
    let dir = tempdir().unwrap();
    let a = write_temp_trace(&dir, "a.json", TRACE_A);
    let b_aarch64 = TRACE_B.replace(r#""arch": "x86_64""#, r#""arch": "aarch64""#);
    let b = write_temp_trace(&dir, "b.json", &b_aarch64);
    AssertCommand::cargo_bin("sandprint")
        .unwrap()
        .args(["profile", "diff"])
        .arg(&a)
        .arg(&b)
        .assert()
        .success();
}

#[test]
#[cfg(target_arch = "x86_64")]
#[ignore = "requires BPF capabilities; run with `cargo test -- --ignored`"]
fn profile_run_traces_witness() {
    let witness = build_witness();
    let dir = tempdir().unwrap();
    let trace_path = dir.path().join("witness.json");

    AssertCommand::cargo_bin("sandprint")
        .unwrap()
        .args(["profile", "run", "--output"])
        .arg(&trace_path)
        .arg("--")
        .arg(&witness)
        .assert()
        .success()
        .stdout(predicate::str::contains("Observed"))
        .stdout(predicate::str::contains("openat"))
        .stdout(predicate::str::contains("write"));

    let trace_content = std::fs::read_to_string(&trace_path).unwrap();
    assert!(trace_content.contains("\"openat\""));
    assert!(trace_content.contains("\"write\""));
    assert!(trace_content.contains("\"exit_group\""));
}

#[cfg(target_arch = "x86_64")]
fn build_witness() -> PathBuf {
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let src = PathBuf::from(&manifest).join("tests/fixtures/syscall_witness.c");
    let out = std::env::temp_dir().join("sandprint_syscall_witness");
    let status = Command::new("cc")
        .args(["-nostdlib", "-static", "-no-pie"])
        .arg(&src)
        .arg("-o")
        .arg(&out)
        .status()
        .expect("invoke cc");
    assert!(status.success(), "failed to compile witness");
    out
}
