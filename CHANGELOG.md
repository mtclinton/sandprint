# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial project scaffold with libbpf-rs and CO-RE BPF skeleton generation.
- eBPF tracer attached to the `sys_enter` raw tracepoint, filtered by PID.
- Process tree following via `sched_process_fork` and cleanup on `sched_process_exit`.
- Per-syscall counter map and ring buffer of observed events.
- Userspace ring buffer consumer with structured logging.
- `profile run` subcommand: launch a command, trace it to completion, print observed syscalls and counts.

[Unreleased]: https://github.com/mtclinton/syscall-profiler/compare/HEAD...HEAD
