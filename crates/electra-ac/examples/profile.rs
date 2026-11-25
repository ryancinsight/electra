//! Measure streaming RC analysis without CSV formatting or frequency-history storage.
use electra_ac::{
    AcError, Capacitance, ElectricConductance, ElectricalImpedance, Frequency, LogSweep, RcLadder,
    Stage, ThermodynamicTemperature,
    noise::{NoiseLadder, ThermalStage},
};
use std::{env, error::Error, hint::black_box, time::Instant};

fn measure(
    grid: LogSweep,
    mut sample: impl FnMut(Frequency) -> Result<f64, AcError>,
) -> Result<(u128, f64), AcError> {
    let start = Instant::now();
    let mut checksum = 0.;
    for f in grid {
        checksum += black_box(sample(f?)?);
    }
    Ok((start.elapsed().as_nanos(), checksum))
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let cells: usize = args.next().ok_or("expected stages")?.parse()?;
    let count: u32 = args.next().ok_or("expected points")?.parse()?;
    let mode = args.next().unwrap_or_else(|| "signal".into());
    let stage = Stage::new(
        ElectricalImpedance::from_base(75.),
        Capacitance::from_base(330e-12),
        ElectricConductance::from_base(1e-6),
    )?;
    let grid = LogSweep::new(
        Frequency::from_base(10.),
        Frequency::from_base(10000.),
        count,
    )?;
    let (elapsed, checksum) = match mode.as_str() {
        "signal" => {
            let stages = vec![stage; cells];
            let ladder = RcLadder::new(black_box(stages.as_slice()))?;
            measure(grid, |f| Ok(ladder.at(f)?.gain.into_base().re))?
        }
        "noise" => {
            let thermal = ThermalStage::new(
                stage,
                ThermodynamicTemperature::from_base(310.),
                ThermodynamicTemperature::from_base(290.),
            )?;
            let stages = vec![thermal; cells];
            let ladder = NoiseLadder::new(black_box(stages.as_slice()))?;
            measure(grid, |f| Ok(ladder.at(f)?.into_base()))?
        }
        _ => return Err("expected signal or noise".into()),
    };
    println!("stages={cells} points={count} elapsed_ns={elapsed} checksum={checksum}");
    Ok(())
}
