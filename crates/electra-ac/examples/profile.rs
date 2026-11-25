//! Measure streaming analysis without CSV formatting or frequency-history storage.
use electra_ac::{
    Capacitance, ElectricConductance, ElectricalImpedance, Frequency, LogSweep, RcLadder, Stage,
};
use std::{env, error::Error, hint::black_box, time::Instant};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let cells: usize = args.next().ok_or("expected stages")?.parse()?;
    let count: u32 = args.next().ok_or("expected points")?.parse()?;
    let stage = Stage::new(
        ElectricalImpedance::from_base(75.),
        Capacitance::from_base(330e-12),
        ElectricConductance::from_base(1e-6),
    )?;
    let stages = vec![stage; cells];
    let ladder = RcLadder::new(black_box(stages.as_slice()))?;
    let start = Instant::now();
    let mut checksum = 0.;
    for f in LogSweep::new(
        Frequency::from_base(10.),
        Frequency::from_base(10000.),
        count,
    )? {
        checksum += black_box(ladder.at(f?)?).gain.into_base().re;
    }
    let elapsed = start.elapsed().as_nanos();
    println!("stages={cells} points={count} elapsed_ns={elapsed} checksum={checksum}");
    Ok(())
}
