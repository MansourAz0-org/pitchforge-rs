//! Detect the pitch of a synthetic plucked tone.
//!
//! `cargo run --example basic --features signals`

use pitchforge::signals::{self, PluckParams};
use pitchforge::{PitchDetector, Yin, YinConfig};

fn main() {
    let sample_rate = 48_000.0;
    let mut yin = Yin::new(sample_rate, 4096, YinConfig::default()).expect("valid config");

    let tone = signals::pluck(146.83, sample_rate, 4096, 0.8, PluckParams::default());
    let estimate = yin.detect(&tone).expect("window has the right length");

    match estimate.frequency() {
        Some(f0) => println!("f0 = {f0:.2} Hz (confidence {:.3})", estimate.confidence),
        None => println!("unvoiced: {:?}", estimate.voicing),
    }
    println!("{:#?}", estimate.diagnostics);
}
