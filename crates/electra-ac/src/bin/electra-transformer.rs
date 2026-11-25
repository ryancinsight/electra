//! Sweep a resistively terminated or open transformer under primary voltage drive.
use electra_ac::{
    Dimensionless, ElectricalImpedance, Frequency, Inductance, LogSweep, PortImpedance,
    magnetics::{
        CoupledInductors, IdealTransformer, MagneticError, SecondaryLoad, TransformerPoint,
    },
};
use std::{
    env,
    error::Error,
    io::{self, Write},
    process::ExitCode,
};

const USAGE: &str = "electra-transformer coupled START_HZ STOP_HZ POINTS LP_H LS_H K LOAD_OHM|open\nelectra-transformer ideal START_HZ STOP_HZ POINTS NS_OVER_NP LOAD_OHM|open";

fn sweep(
    grid: LogSweep,
    at: impl Fn(Frequency) -> Result<TransformerPoint, MagneticError>,
) -> Result<(), Box<dyn Error>> {
    let mut out = io::BufWriter::new(io::stdout().lock());
    writeln!(
        out,
        "frequency_Hz,gain_re,gain_im,input_port,zin_re_ohm,zin_im_ohm"
    )?;
    for f in grid {
        let f = f?;
        let point = at(f)?;
        let g = point.gain.into_base();
        write!(out, "{},{},{},", f.into_base(), g.re, g.im)?;
        match point.input_impedance {
            PortImpedance::Open => writeln!(out, "open,,")?,
            PortImpedance::Finite(z) => {
                writeln!(out, "finite,{},{}", z.as_base().re, z.as_base().im)?
            }
        }
    }
    out.flush()?;
    Ok(())
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mode = args.next().ok_or(USAGE)?;
    if mode == "--help" {
        println!("{USAGE}");
        return Ok(());
    }
    let mut next = || args.next().ok_or(USAGE);
    let grid = LogSweep::new(
        Frequency::from_base(next()?.parse()?),
        Frequency::from_base(next()?.parse()?),
        next()?.parse()?,
    )?;
    let finish = |value: String, extra: Option<String>| -> Result<SecondaryLoad, Box<dyn Error>> {
        if extra.is_some() {
            return Err(USAGE.into());
        }
        Ok(if value == "open" {
            SecondaryLoad::Open
        } else {
            SecondaryLoad::Resistance(ElectricalImpedance::from_base(value.parse()?))
        })
    };
    match mode.as_str() {
        "coupled" => {
            let model = CoupledInductors::new(
                Inductance::from_base(next()?.parse()?),
                Inductance::from_base(next()?.parse()?),
                Dimensionless::from_base(next()?.parse()?),
                finish(next()?, args.next())?,
            )?;
            sweep(grid, |f| model.at(f))
        }
        "ideal" => {
            let model = IdealTransformer::new(
                Dimensionless::from_base(next()?.parse()?),
                finish(next()?, args.next())?,
            )?;
            sweep(grid, |f| model.at(f))
        }
        _ => Err(USAGE.into()),
    }
}

fn main() -> ExitCode {
    if let Err(error) = run() {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
