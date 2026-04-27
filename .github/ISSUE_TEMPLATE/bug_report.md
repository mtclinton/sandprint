---
name: Bug report
about: Report a defect in sandprint
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

- sandprint version: `sandprint --version`
- Kernel: `uname -a`
- Distribution: `cat /etc/os-release | head -2`
- BTF available: `ls -la /sys/kernel/btf/vmlinux`
- Architecture: `uname -m`

## Verbose output

<!-- Re-run with `RUST_LOG=sandprint=debug` and paste the output. -->

```text

```
