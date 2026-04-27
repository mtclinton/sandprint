use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use libbpf_cargo::SkeletonBuilder;

const BPF_SRC: &str = "src/bpf/tracer.bpf.c";

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is always set by cargo"));
    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is always set by cargo"),
    );

    let bpf_src = manifest_dir.join(BPF_SRC);
    let skel_out = out_dir.join("tracer.skel.rs");

    let mut clang_args: Vec<String> = vec![
        "-I".into(),
        manifest_dir.join("src/bpf").to_string_lossy().into_owned(),
        "-Wno-unused-function".into(),
    ];

    // On Debian-family systems, kernel UAPI headers live under a
    // multiarch path (e.g., /usr/include/x86_64-linux-gnu/asm/types.h).
    // clang doesn't search that path by default, so we add it explicitly.
    // On Fedora-family systems the asm/ headers live directly under
    // /usr/include and the loop is a no-op.
    for inc in detect_multiarch_includes() {
        if Path::new(&inc).join("asm/types.h").exists() {
            clang_args.push("-I".into());
            clang_args.push(inc);
        }
    }

    SkeletonBuilder::new()
        .source(&bpf_src)
        .clang_args(clang_args)
        .build_and_generate(&skel_out)
        .expect("failed to compile BPF program and generate skeleton");

    println!("cargo:rerun-if-changed={}", bpf_src.display());
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("src/bpf/tracer.h").display()
    );
}

fn detect_multiarch_includes() -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(output) = Command::new("gcc").arg("-print-multiarch").output() {
        if let Ok(s) = String::from_utf8(output.stdout) {
            let triple = s.trim();
            if !triple.is_empty() {
                out.push(format!("/usr/include/{triple}"));
            }
        }
    }
    out.push("/usr/include/x86_64-linux-gnu".to_string());
    out.push("/usr/include/aarch64-linux-gnu".to_string());
    out
}
