//! Implementation of `profile attach`. Loads the BPF tracer, registers
//! the user-supplied PID, polls the ring buffer for the requested
//! duration, and prints a summary.
//!
//! Children spawned by the target during the attach window are picked
//! up automatically by the BPF `sched_process_fork` handler — no extra
//! bookkeeping is needed in userspace.

use std::mem::MaybeUninit;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use libbpf_rs::{OpenObject, RingBufferBuilder};
use tracing::info;

use crate::cli::AttachArgs;
use crate::tracer::output::{build_trace, print_summary};
use crate::tracer::ringbuf::SyscallEvent;
use crate::tracer::{Tracer, TracerConfig};

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

pub fn run(args: AttachArgs) -> Result<()> {
    let config = TracerConfig::default();
    let mut object: MaybeUninit<OpenObject> = MaybeUninit::uninit();
    let mut tracer = Tracer::new(&mut object, config).context("failed to load BPF tracer")?;
    info!("BPF tracer loaded and attached");

    tracer.track_pid(args.pid)?;
    info!(pid = args.pid, "tracking existing PID");

    let _ = ctrlc::set_handler(|| SHUTDOWN.store(true, Ordering::SeqCst));

    let collected: Arc<Mutex<Vec<SyscallEvent>>> = Arc::new(Mutex::new(Vec::with_capacity(4096)));
    let collected_for_cb = Arc::clone(&collected);

    let mut builder = RingBufferBuilder::new();
    builder
        .add(&tracer.skel().maps.events, move |bytes: &[u8]| -> i32 {
            if bytes.len() >= std::mem::size_of::<SyscallEvent>() {
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

    let total = Duration::from_secs(args.duration);
    let deadline = Instant::now() + total;
    info!(duration_secs = args.duration, "tracing for duration");

    while !SHUTDOWN.load(Ordering::Relaxed) {
        let now = Instant::now();
        if now >= deadline {
            break;
        }
        let timeout = (deadline - now).min(tracer.config().poll_interval);
        match rb.poll(timeout) {
            Ok(_) => {}
            Err(e) if e.kind() == libbpf_rs::ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
        }
    }

    for _ in 0..3 {
        let _ = rb.poll(Duration::from_millis(50));
    }

    let events = collected.lock().expect("collected mutex poisoned");
    let counts = tracer.syscall_counts()?;

    print_summary(events.len(), &counts);

    if let Some(path) = args.output.as_deref() {
        let argv = vec![format!("pid:{}", args.pid)];
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
