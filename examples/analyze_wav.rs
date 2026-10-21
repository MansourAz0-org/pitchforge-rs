//! Track the pitch of a WAV file and print one line per analysis hop.
//!
//! ```text
//! cargo run --release --example analyze_wav -- recording.wav [window] [hop]
//! ```
//!
//! Multi-channel files are mixed down to mono. Output columns:
//! time (s), f0 (Hz or `-`), confidence, RMS (dBFS), voicing.

use pitchforge::{PitchDetector, Yin, YinConfig};
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let Some(path) = args.get(1) else {
        eprintln!("usage: analyze_wav <file.wav> [window=4096] [hop=512]");
        return ExitCode::FAILURE;
    };
    let window: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4096);
    let hop: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(512);

    let mut reader = match hound::WavReader::open(path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("cannot open {path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let spec = reader.spec();
    let channels = spec.channels as usize;
    let interleaved: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().map(Result::unwrap).collect(),
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1u64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.unwrap() as f32 * scale)
                .collect()
        }
    };
    let mono: Vec<f32> = interleaved
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect();

    let sample_rate = spec.sample_rate as f64;
    let mut yin = match Yin::new(sample_rate, window, YinConfig::default()) {
        Ok(y) => y,
        Err(e) => {
            eprintln!("invalid configuration: {e}");
            return ExitCode::FAILURE;
        }
    };

    println!(
        "# {path}: {} Hz, {channels} ch, window {window}, hop {hop}",
        spec.sample_rate
    );
    println!("# time_s\tf0_hz\tconfidence\trms_dbfs\tvoicing");
    let mut start = 0;
    while start + window <= mono.len() {
        let est = yin
            .detect(&mono[start..start + window])
            .expect("window length");
        let t = (start + window / 2) as f64 / sample_rate;
        let f0 = est
            .frequency()
            .map_or_else(|| "-".to_string(), |f| format!("{f:.2}"));
        let db = 20.0 * est.diagnostics.rms.log10();
        println!(
            "{t:.3}\t{f0}\t{:.3}\t{db:.1}\t{:?}",
            est.confidence, est.voicing
        );
        start += hop;
    }
    ExitCode::SUCCESS
}
