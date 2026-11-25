//! Compare reusing a prepared transformer with preparing it separately for every sample.
use electra_ac::{
    Dimensionless, ElectricalImpedance, Frequency, Inductance, LogSweep, PortImpedance,
    magnetics::{CoupledInductors, MagneticError, SecondaryLoad, TransformerPoint},
};
use std::{error::Error, hint::black_box, time::Instant};

fn measure(
    grid: LogSweep,
    at: impl Fn(Frequency) -> Result<TransformerPoint, MagneticError>,
) -> Result<(u128, f64), Box<dyn Error>> {
    let start = Instant::now();
    let mut checksum = 0.;
    for f in grid {
        let point = black_box(at(f?)?);
        if let PortImpedance::Finite(z) = point.input_impedance {
            checksum += point.gain.as_base().re + z.as_base().re;
        }
    }
    Ok((start.elapsed().as_nanos(), checksum))
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mode = args.next().ok_or("expected prepared or rebuild")?;
    let count = args.next().ok_or("expected point count")?.parse()?;
    let prepare = || {
        let (lp, ls, k, r) = black_box((0.001, 0.004, 0.97, 47.));
        CoupledInductors::new(
            Inductance::from_base(lp),
            Inductance::from_base(ls),
            Dimensionless::from_base(k),
            SecondaryLoad::Resistance(ElectricalImpedance::from_base(r)),
        )
    };
    let grid = LogSweep::new(Frequency::from_base(1.), Frequency::from_base(1e8), count)?;
    let (ns, checksum) = match mode.as_str() {
        "prepared" => {
            let model = black_box(prepare()?);
            measure(grid, |f| model.at(f))?
        }
        "rebuild" => measure(grid, |f| prepare()?.at(f))?,
        _ => return Err("expected prepared or rebuild".into()),
    };
    println!("mode={mode} points={count} elapsed_ns={ns} checksum={checksum}");
    Ok(())
}
