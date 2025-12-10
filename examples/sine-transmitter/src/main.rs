use futuresdr::blocks::SignalSourceBuilder;
use futuresdr::blocks::seify::SinkBuilder;
use futuresdr::macros::connect;
use futuresdr::num_complex::Complex32;
use futuresdr::runtime::{Flowgraph, Runtime};

fn main() -> anyhow::Result<()> {
    // Create the `Flowgraph` where the `Block`s will be added later on
    let mut fg = Flowgraph::new();

    let src = SignalSourceBuilder::<Complex32>::sin(450.0, 3000.0).build();

    // Create a new Seify SDR block with the given parameters
    let snk = SinkBuilder::new()
        .frequency(1_000_000_000.0)
        .sample_rate(5_000_000.0)
        .gain(45.0)
        .args("driver=bladerf")? // "driver=soapy,xb200=auto"
        .build()?;

    // Add all the blocks to the `Flowgraph`...
    connect!(fg, src > snk;);

    Runtime::new().run(fg)?;

    Ok(())
}
