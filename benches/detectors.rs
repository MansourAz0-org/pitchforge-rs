//! Per-window processing time of the pitch detectors.
//!
//! `cargo bench --bench detectors`

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use pitchforge::signals::{self, PluckParams};
use pitchforge::{DifferenceMethod, PitchDetector, Yin, YinConfig};
use std::hint::black_box;

const SR: f64 = 48_000.0;

fn yin(c: &mut Criterion) {
    let mut group = c.benchmark_group("yin");
    for window in [2048usize, 4096, 8192] {
        // A low oud course (Do2 lowered by 2 semitones) — worst case for the
        // lag range — at the default 50–1400 Hz search range.
        let sig = signals::pluck(58.27, SR, window, 0.8, PluckParams::default());
        group.throughput(Throughput::Elements(window as u64));
        for (label, method) in [
            ("direct", DifferenceMethod::Direct),
            ("fft", DifferenceMethod::Fft),
        ] {
            let config = YinConfig {
                difference: method,
                ..YinConfig::default()
            };
            let mut det = Yin::new(SR, window, config).unwrap();
            group.bench_with_input(BenchmarkId::new(label, window), &sig, |b, sig| {
                b.iter(|| det.detect(black_box(sig)).unwrap())
            });
        }
    }
    let silence = vec![0.0f32; 4096];
    let mut det = Yin::new(SR, 4096, YinConfig::default()).unwrap();
    group.bench_function("silence-gate/4096", |b| {
        b.iter(|| det.detect(black_box(&silence)).unwrap())
    });
    group.finish();
}

criterion_group!(benches, yin);
criterion_main!(benches);
