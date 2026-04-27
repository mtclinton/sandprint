# Security policy

## Supported versions

This project is pre-1.0. Only the latest release on `main` is supported with
security fixes.

## Reporting a vulnerability

Please **do not** open a public issue for security-sensitive reports.
Instead, send a description of the issue and reproduction steps to the
maintainer privately. GitHub's "Report a vulnerability" workflow on the
repository's Security tab is the preferred channel.

You can expect an acknowledgement within 72 hours and a coordinated
disclosure timeline thereafter.

## Threat model

sandprint runs an eBPF program in the kernel. Loading eBPF requires
`CAP_BPF + CAP_PERFMON` (or `CAP_SYS_ADMIN`); the binary itself does not
elevate privilege beyond that. The eBPF verifier rejects unsafe programs;
the bundled BPF program is small, bounded, and contains no helper calls
that allow kernel writes.

The userspace consumer parses ring buffer events whose layout is defined
by the bundled BPF program. Trust boundary is the kernel: events read from
the ring buffer are treated as authoritative.

The generated seccomp profiles are based on observed behavior during a
profiling run. They are only as complete as the test workload that
exercises the target. Always review generated profiles before applying
them in production.
