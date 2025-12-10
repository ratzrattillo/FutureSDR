//! A simple FM receiver that you can tune to nearby radio stations
//!
//! When you run the example, it will build a flowgraph consisting of the following blocks:
//! * SeifySource: Gets data from your SDR
//! * Demodulator: Demodulates the FM signal
//! * AudioSink: Plays the demodulated signal on your device
//!
//! After giving it some time to start up the SDR, it enters a loop where you will
//! be periodically asked to enter a new frequency that the SDR will be tuned to.
//! **Watch out** though: Some frequencies (very high or very low) might be unsupported
//! by your SDR and may cause a crash.

use anyhow::Result;
use clap::Parser;
use futuresdr::blocks::{Apply, Combine, Delay, FirBuilder, SignalSourceBuilder, Split};
//use futuresdr::async_io;
// use futuresdr::blocks::audio::AudioSink;
use futuresdr::blocks::audio::FileSource;
// use futuresdr::blocks::seify::SourceBuilder;
// use futuresdr::blocks::Apply;
// use futuresdr::blocks::FirBuilder;
// use futuresdr::futuredsp::firdes;
use futuresdr::macros::connect;
//use futuresdr::num_complex::Complex32;
//use futuresdr::num_integer::gcd;
use futuresdr::runtime::Flowgraph;
//use futuresdr::runtime::Pmt;
use futuresdr::blocks::seify::SinkBuilder;
use futuresdr::futuredsp::firdes;
use futuresdr::futuredsp::windows::hamming;
use futuresdr::num_complex::Complex;
use futuresdr::runtime::Runtime;

#[derive(Parser, Debug)]
struct Args {
    /// Gain to apply to the seify source
    #[clap(short, long, default_value_t = 30.0)]
    gain: f64,

    /// Center frequency
    #[clap(short, long, default_value_t = 107_000_000.0)]
    frequency: f64,

    /// Sample rate
    #[clap(short, long, default_value_t = 1000000.0)]
    sample_rate: f64,

    /// Seify args
    #[clap(short, long, default_value = "")]
    args: String,
    // /// Multiplier for intermedia sample rate
    // #[clap(long)]
    // audio_mult: Option<u32>,

    // /// Audio Rate
    // #[clap(long)]
    // audio_rate: Option<u32>,
}

fn main() -> Result<()> {
    // cargo run --release -- --frequency=107700000 --gain=45.0 --rate=5000000 --args "driver=bladerf"
    // Why am i at 107.7 when i enter 98.5?: When the sample rate is unsupported, this happens...
    // cargo run --release -- --frequency=107700000 --gain=30.0 --rate=1800000 --args "driver=rtlsdr"
    futuresdr::runtime::init();

    let args = Args::parse();
    println!("Configuration {args:?}");

    // Create the `Flowgraph` where the `Block`s will be added later on
    let mut fg = Flowgraph::new();

    let src = FileSource::new("rick.mp3");
    let file_sample_rate = src.kernel.sample_rate();
    let file_channels = src.kernel.channels();
    println!("File details: Sample rate: {file_sample_rate}, Channels: {file_channels}");

    println!(
        "Transmitting on Frequency: {}Hz, Sample Rate: {}Hz, Gain: {}dB",
        args.frequency, args.sample_rate, args.gain
    );

    let split = Split::new(move |v: &f32| (*v, *v));

    // Phase transformation by 90°.
    let window = hamming(167, false);
    let taps = firdes::hilbert(window.as_slice());
    let hilbert = FirBuilder::new::<f32, f32, _>(taps);

    // Match the delay caused by the phase transformation.
    let delay = Delay::<f32>::new(window.len() as isize / -2);

    let to_complex = Combine::new(move |i: &f32, q: &f32| Complex::<f32>::new(*i, *q));

    // let carrier_src = SignalSourceBuilder::<Complex<f32>>::sin(98.5e6, 200e3);
    //
    // let to_carrier_freq = Combine::new(move |carrier_src: &Complex<f32>, sample: &Complex<f32>|  {
    //     *carrier * *sample
    // });
    /// Modulates a complex baseband signal to a specific carrier frequency.
    ///
    /// # Arguments
    /// * `baseband_signal` - The input complex baseband signal.
    /// * `carrier_frequency` - The desired carrier frequency (Hz).
    /// * `sample_rate` - The sample rate of the signal (Hz).
    // fn modulate_signal(
    //     baseband_signal: &[Complex<f32>],
    //     carrier_frequency: f32,
    //     sample_rate: f32,
    // ) -> Vec<Complex<f32>> {
    //     baseband_signal
    //         .iter()
    //         .enumerate()
    //         .map(|(n, &sample)| {
    //             // Calculate the phase of the carrier wave for the current sample
    //             let time = n as f32 / sample_rate;
    //             let phase = std::f32::consts::TAU * carrier_frequency * time;
    //
    //             // Create the complex carrier wave: exp(j*phi) = cos(phi) + j*sin(phi)
    //             let carrier = Complex::<f32>::from_polar(1.0, phase); // from_polar(magnitude, angle)
    //
    //             // Mix the signal by complex multiplication
    //             sample * carrier
    //         })
    //         .collect()
    // }

    // Create a new Seify SDR block with the given parameters
    let snk = SinkBuilder::new()
        .frequency(args.frequency)
        .sample_rate(args.sample_rate)
        .gain(args.gain)
        // .args(args.args.unwrap_or_else(String::new))?
        .build()?;

    // Add all the blocks to the `Flowgraph`...
    connect!(fg, src > split;
        split.out0 > delay > to_complex.in0;
        split.out1 > hilbert > to_complex.in1;
        to_complex > snk;);

    Runtime::new().run(fg)?;

    Ok(())
}
