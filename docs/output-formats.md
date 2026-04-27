# Output formats

`profile generate --format <fmt>` selects an emitter. This document
describes each format and the schema of the canonical JSON the
emitters consume.

## Canonical JSON schema

```json
{
  "schema_version": 1,
  "arch": "x86_64",
  "target_argv": ["curl", "-s", "https://example.com"],
  "events": [
    {
      "timestamp_ns": 1234567890,
      "tgid": 1234,
      "tid": 1234,
      "syscall_nr": 257,
      "syscall_name": "openat",
      "comm": "curl"
    }
  ],
  "counts": [
    { "syscall_nr": 257, "syscall_name": "openat", "count": 12 }
  ]
}
```

Notes:

- `events` is per-event detail. `profile merge` drops it because
  per-event detail does not compose meaningfully across runs.
- `counts` is sorted by descending count.
- `syscall_name` is `null` when the number does not correspond to a
  known syscall on `arch`. Emitters that need names (OCI, systemd,
  seccomp-h) skip these entries.
- `schema_version` is bumped on incompatible layout changes; older
  versions stay readable as `profile generate` adds a translation
  layer when the bump happens.

## OCI seccomp (`--format oci`)

Produces the `linux.seccomp` block of an OCI runtime spec:

```json
{
  "defaultAction": "SCMP_ACT_ERRNO",
  "defaultErrnoRet": 1,
  "architectures": ["SCMP_ARCH_X86_64"],
  "syscalls": [
    { "names": ["openat", "read", "..."], "action": "SCMP_ACT_ALLOW" }
  ]
}
```

The default action is `SCMP_ACT_ERRNO(1)` rather than
`SCMP_ACT_KILL` so that unobserved syscalls return EPERM to the
application. Easier to spot and recover from in production than a
mysterious SIGSYS.

Currently supported `arch` values: `x86_64`, `aarch64`. Other
architectures return `ProfileError::UnsupportedArch`.

## systemd (`--format systemd`)

A single line suitable for inclusion in a `.service` unit:

```
SystemCallFilter=read write openat close mmap mprotect ...
```

systemd's default action for `SystemCallFilter=` is to terminate
the process with SIGSYS for unobserved calls, the inverse of the
OCI emitter. If you need EPERM behavior, add
`SystemCallErrorNumber=EPERM` to the unit.

## libseccomp C header (`--format seccomp-h`)

```c
#include <seccomp.h>

static const int sandprint_syscalls[] = {
    SCMP_SYS(openat),
    SCMP_SYS(read),
    /* ... */
};

#define SANDPRINT_NUM_SYSCALLS 21
```

Use it like:

```c
scmp_filter_ctx ctx = seccomp_init(SCMP_ACT_ERRNO(1));
for (size_t i = 0; i < SANDPRINT_NUM_SYSCALLS; i++) {
    seccomp_rule_add(ctx, SCMP_ACT_ALLOW, sandprint_syscalls[i], 0);
}
seccomp_load(ctx);
```

## Markdown (`--format markdown`)

Human-readable report with a table of syscall counts. Useful for
PR descriptions or design docs that explain a profile's contents.

## JSON passthrough (`--format json`)

Re-emits the canonical trace as pretty-printed JSON. Useful as a
round-trip check or for sorting/filtering before re-feeding the
trace to another emitter.
