//! Benchmark for the userspace ring buffer consumer's hot path.
//!
//! What this measures: per-event work in userspace once a payload has
//! been delivered by the kernel — `read_unaligned` of the wire-format
//! event, plus pushing it onto a `Vec`. It does not measure the
//! kernel-side `bpf_ringbuf_reserve` / `bpf_ringbuf_submit` cost or
//! the `epoll_wait` syscall in `RingBuffer::poll`; those depend on
//! kernel and workload and would need a process under load to bench
//! meaningfully.
//!
//! The benchmark runs across a few payload sizes so a regression in
//! per-event work shows up as a delta on every input size, while a
//! regression in setup cost (allocation, Vec growth) is more visible
//! on the smaller inputs.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sandprint::tracer::ringbuf::SyscallEvent;

const EVENT_SIZE: usize = std::mem::size_of::<SyscallEvent>();

fn make_payload(num_events: usize) -> Vec<u8> {
    let mut buf = vec![0u8; num_events * EVENT_SIZE];
    for i in 0..num_events {
        let off = i * EVENT_SIZE;
        // Stamp `tgid` and `syscall_nr` with deterministic values so
        // the optimizer doesn't fold the read away.
        buf[off + 8..off + 12].copy_from_slice(&((i as u32).wrapping_mul(7)).to_ne_bytes());
        buf[off + 16..off + 20].copy_from_slice(&((i as u32) % 512).to_ne_bytes());
    }
    buf
}

fn parse_and_collect(payload: &[u8]) -> Vec<SyscallEvent> {
    let mut events = Vec::with_capacity(payload.len() / EVENT_SIZE);
    for chunk in payload.chunks_exact(EVENT_SIZE) {
        // SAFETY: `chunk.len() == EVENT_SIZE` and `SyscallEvent` is
        // `#[repr(C)]` with no invalid bit patterns — every byte
        // sequence of the right length is a valid event.
        let evt =
            unsafe { std::ptr::read_unaligned(chunk.as_ptr() as *const SyscallEvent) };
        events.push(evt);
    }
    events
}

fn bench_consumer(c: &mut Criterion) {
    let mut group = c.benchmark_group("ringbuf_consumer");
    for &n in &[64usize, 1_024, 16_384] {
        let payload = make_payload(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &payload, |b, payload| {
            b.iter(|| {
                let events = parse_and_collect(black_box(payload));
                black_box(events);
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_consumer);
criterion_main!(benches);
