# Contributing

Thanks for your interest in syscall-profiler. This document describes how to
get a local development environment up and running, and how to send changes
upstream.

## Reporting bugs and requesting features

Use the issue templates under `.github/ISSUE_TEMPLATE/`. For bug reports,
include kernel version (`uname -a`), distribution, the exact invocation that
triggered the bug, and the output of running with `RUST_LOG=syscall_profiler=debug`.

For security issues, see [SECURITY.md](SECURITY.md).

## Development environment

You need a recent Linux kernel (5.8+ for ringbuf, BTF enabled), clang, libelf,
and a stable Rust toolchain. The pinned toolchain in `rust-toolchain.toml` will
be installed automatically by `rustup`.

```sh
sudo apt install -y clang llvm libelf1 libelf-dev linux-libc-dev
./scripts/install-deps.sh   # optional; convenience wrapper
cargo build
```

The eBPF C source under `src/bpf/` is compiled at build time by `build.rs`
via `libbpf-cargo`. The resulting BPF object is embedded into the binary,
so the released binary is a single file with no runtime dependency on
external `.bpf.o` artifacts.

## Running locally

Tracing requires either `CAP_BPF + CAP_PERFMON` or the `CAP_SYS_ADMIN`
fallback. The simplest way to grant these to a development build:

```sh
sudo setcap 'cap_bpf,cap_perfmon=eip' target/debug/syscall-profiler
target/debug/syscall-profiler profile run -- ls /tmp
```

## Quality gates

Before opening a pull request, run:

```sh
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

CI runs the same checks across the supported MSRV and platforms in the
matrix; if your local toolchain disagrees with CI on style, the CI version
wins.

## Commit style

Commits use [Conventional Commits](https://www.conventionalcommits.org/).
Examples:

- `feat(tracer): follow children via sched_process_fork`
- `fix(cli): handle SIGINT during profile run`
- `docs(readme): add comparison table`

Keep commits focused. A commit that mixes a refactor and a feature is a
commit that should have been two.

## Style notes

- Library code raises `thiserror`-derived errors; the binary uses `anyhow`.
- Logging is via `tracing`. Spans are preferred over manual context strings
  in error messages where the span already encodes the context.
- Don't add comments that just restate the code. Comments should explain
  *why*, not *what*.
- No emoji in commit messages, code, or documentation.

## License

By contributing, you agree that your contributions will be licensed under
the Apache License, Version 2.0.
