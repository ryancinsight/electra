//! Time preparation and streaming separately on an already parsed deck.
use electra_circuit::{Circuit, MnaOptions, direct::DirectFactor};
use std::{error::Error, hint::black_box, time::Instant};

pub(super) fn run<F: DirectFactor>(c: &Circuit, limits: MnaOptions) -> Result<(), Box<dyn Error>> {
    let start = Instant::now();
    let mut sweep = c.dc_sweep_direct::<F>(limits)?;
    let prepared = start.elapsed();
    let mut points = 0;
    while let Some(sample) = sweep.next_point()? {
        black_box(sample.result);
        points += 1;
    }
    let elapsed = start.elapsed();
    println!(
        "points={points} prepare_us={} total_us={}",
        prepared.as_micros(),
        elapsed.as_micros()
    );
    Ok(())
}
