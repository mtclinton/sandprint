---
name: Bug report
about: Report a defect in syscall-profiler
title: ''
labels: bug
assignees: ''
---

## Description

<!-- Clear, terse description of the bug. -->

## Reproduction

```sh
# Exact command(s) that triggered the bug.
```

## Expected behavior

<!-- What did you expect to happen? -->

## Actual behavior

<!-- What happened instead? Include relevant output. -->

## Environment

- syscall-profiler version: `syscall-profiler --version`
- Kernel: `uname -a`
- Distribution: `cat /etc/os-release | head -2`
- BTF available: `ls -la /sys/kernel/btf/vmlinux`
- Architecture: `uname -m`

## Verbose output

<!-- Re-run with `RUST_LOG=syscall_profiler=debug` and paste the output. -->

```text

```
