# sandprint architecture

This document expands on the mermaid diagram in the [README](../README.md)
and walks through how each piece fits together.

## Module map

```
src/
├── main.rs                # entry point: clap parse + dispatch
├── lib.rs                 # crate root
├── cli.rs                 # subcommand definitions and routing
├── tracer/                # BPF loading + ring buffer consumption
│   ├── mod.rs             # Tracer struct, lifetime-bound to OpenObject
│   ├── attach.rs          # spawn-suspended fork+exec for `profile run`
│   ├── attach_pid.rs      # `profile attach` against an existing PID
│   ├── output.rs          # shared print/build helpers
│   ├── pidtree.rs         # snapshot of tracked PIDs
│   ├── ringbuf.rs         # SyscallEvent wire layout
│   └── run.rs             # `profile run` implementation
├── profile/               # canonical Trace + emitters
│   ├── mod.rs             # Trace, Format, ProfileError, emit, load_trace
│   ├── trace.rs           # canonical Trace data type
│   ├── oci.rs             # OCI runtime spec emitter
│   ├── systemd.rs         # SystemCallFilter= emitter
│   ├── seccomp_h.rs       # libseccomp C header emitter
│   ├── markdown.rs        # human-readable report emitter
│   ├── diff.rs            # symmetric set diff
│   └── merge.rs           # union of multiple traces
├── syscalls/              # syscall number → name resolution
└── bpf/                   # the eBPF program (compiled via build.rs)
    ├── tracer.bpf.c
    └── tracer.h
```

## Lifecycle of `profile run`

```
1. main → CLI parse → tracer::run::run(args)
2. Tracer::new
   ├── setrlimit RLIMIT_MEMLOCK = INFINITY (best-effort)
   ├── TracerSkelBuilder::open(&mut OpenObject)
   ├── OpenSkel::load                  # verifier runs here
   └── Skel::attach                    # raw tracepoints attached
3. spawn_suspended(argv)
   ├── pipe2(O_CLOEXEC) for the sync handshake
   ├── fork
   ├── child: read(sync_fd, 1)         # blocks until parent unblocks
   └── parent: returns ChildHandle{ pid, write_fd }
4. tracer.track_pid(child.pid)         # update BPF hashmap
5. RingBufferBuilder::add(&events_map, callback) → build → poll loop
6. child.release()                     # 1 byte over pipe → child execvp's
7. main loop: rb.poll(timeout); waitpid(child, WNOHANG)
8. child exit → drain ringbuf → print summary → optional JSON write
```

The drop order at the end of the function matters. `rb` borrows the
events map from `tracer`, which borrows from `object` (the
`MaybeUninit<OpenObject>` stack slot). Locals are dropped in reverse
declaration order, so `rb` is dropped before `tracer` which is
dropped before `object`. The pattern is documented on
[`tracer::Tracer::new`](../src/tracer/mod.rs).

## `profile run` versus `profile attach`

Both share the same skeleton load, ring buffer consume, and summary
print. They differ at the front:

| Step                | run                                | attach                            |
| ------------------- | ---------------------------------- | --------------------------------- |
| PID acquisition     | fork + execvp via spawn_suspended  | --pid from CLI                    |
| Tracking start      | after fork, before execvp          | immediately                       |
| Stop condition      | waitpid for child exit             | --duration elapsed or SIGINT      |
| Process tree follow | yes (descendants of the child)     | yes (descendants of --pid)        |

The BPF side is identical. `tracked_pids` is just a hashmap keyed by
TGID; the `sched_process_fork` handler admits children whose parent
is in the set, regardless of where membership came from.

## Profile pipeline

```
profile run | profile attach
       ↓ (canonical JSON on disk)
       Trace
       ↓
profile generate (json | oci | systemd | seccomp-h | markdown)
profile diff/merge (operates on multiple Traces)
```

The canonical [`Trace`](../src/profile/trace.rs) is the boundary
between the tracer and the emitters. It serializes as JSON; the
emitters take a `Trace` and produce a `String` in the chosen
format. Because the tracer always produces a Trace and the emitters
only read one, the two halves can evolve independently.

## Privileges

Loading the BPF program requires either `CAP_BPF` + `CAP_PERFMON`
(kernels 5.8+) or `CAP_SYS_ADMIN`. Bumping `RLIMIT_MEMLOCK`
additionally needs `CAP_SYS_RESOURCE` on older kernels; on modern
kernels (5.11+) the bump is a no-op for BPF maps and the warning
sandprint logs is benign.

The recommended local setup is `setcap`:

```sh
sudo setcap 'cap_bpf,cap_perfmon=eip' /usr/local/bin/sandprint
```

The trap on common Linux distributions: `/tmp` is mounted with
`nosuid`, which silently disables file capabilities. Install the
binary somewhere else (`~/`, `/usr/local/bin`) before `setcap`-ping.
