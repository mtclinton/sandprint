//! Implementation of `profile run`. Loads the BPF tracer, spawns the
//! target process suspended, registers it with the tracker, releases
//! the child, and consumes events until the child exits.

use std::mem::MaybeUninit;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use libbpf_rs::{OpenObject, RingBufferBuilder};
use nix::sys::signal::{kill, Signal};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use tracing::{info, warn};

use crate::cli::RunArgs;
use crate::tracer::attach::spawn_suspended;
use crate::tracer::output::{build_trace, print_summary};
use crate::tracer::ringbuf::SyscallEvent;
use crate::tracer::{Tracer, TracerConfig};

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

pub fn run(args: RunArgs) -> Result<()> {
    let config = TracerConfig {
        include_children: !args.no_children,
        ..Default::default()
    };

    // Storage for the BPF object backing the skeleton. Must be declared
    // before `tracer` so it outlives it (drop order is reverse of decl).
    let mut object: MaybeUninit<OpenObject> = MaybeUninit::uninit();
    let mut tracer = Tracer::new(&mut object, config).context("failed to load BPF tracer")?;
    info!("BPF tracer loaded and attached");

    let _ = ctrlc::set_handler(|| SHUTDOWN.store(true, Ordering::SeqCst));

    let child = spawn_suspended(&args.command).context("failed to spawn target")?;
    let child_pid = child.pid;
    tracer.track_pid(child_pid.as_raw() as u32)?;
    info!(pid = child_pid.as_raw(), "tracing command");

    let collected: Arc<Mutex<Vec<SyscallEvent>>> = Arc::new(Mutex::new(Vec::with_capacity(1024)));
    let collected_for_cb = Arc::clone(&collected);

    let mut builder = RingBufferBuilder::new();
    builder
        .add(&tracer.skel().maps.events, move |bytes: &[u8]| -> i32 {
            if bytes.len() >= std::mem::size_of::<SyscallEvent>() {
                // SAFETY: BPF guarantees the ring buffer payload begins
                // with a properly initialized `syscall_event`. We use
                // `read_unaligned` to be defensive about alignment.
                let evt =
                    unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const SyscallEvent) };
                if let Ok(mut v) = collected_for_cb.lock() {
                    v.push(evt);
                }
            }
            0
        })
        .context("ringbuf add")?;
    let rb = builder.build().context("ringbuf build")?;

    child.release().context("releasing child")?;

    let timeout = tracer.config().poll_interval;
    let mut child_alive = true;
    let mut signaled_term = false;
    while child_alive {
        if SHUTDOWN.load(Ordering::Relaxed) && !signaled_term {
            warn!("interrupted; sending SIGTERM to child");
            let _ = kill(child_pid, Signal::SIGTERM);
            signaled_term = true;
        }
        match rb.poll(timeout) {
            Ok(_) => {}
            Err(e) if e.kind() == libbpf_rs::ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
        }
        match waitpid(child_pid, Some(WaitPidFlag::WNOHANG))? {
            WaitStatus::StillAlive => {}
            status => {
                info!(?status, "child exited");
                child_alive = false;
            }
        }
    }

    for _ in 0..3 {
        let _ = rb.poll(Duration::from_millis(50));
    }

    let events = collected.lock().expect("collected mutex poisoned");
    let counts = tracer.syscall_counts()?;

    print_summary(events.len(), &counts);

    if let Some(path) = args.output.as_deref() {
        let argv: Vec<String> = args
            .command
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        write_trace(path, &argv, &events, &counts)?;
        info!(path = %path.display(), "wrote trace");
    }

    Ok(())
}

fn write_trace(
    path: &Path,
    argv: &[String],
    events: &[SyscallEvent],
    counts: &[(u32, u64)],
) -> Result<()> {
    let trace = build_trace(argv, events, counts);
    let f = std::fs::File::create(path).context("create trace file")?;
    serde_json::to_writer_pretty(f, &trace).context("serialize trace")?;
    Ok(())
}
