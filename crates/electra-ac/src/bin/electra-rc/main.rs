//! Stream the voltage response or thermal noise of a passive RC ladder.
mod output;
use electra_ac::{
    Capacitance, ElectricConductance, ElectricalImpedance, Frequency, LogSweep, RcLadder, Stage,
    ThermodynamicTemperature,
    noise::{NoiseLadder, ThermalStage},
};
use std::{
    env,
    error::Error,
    io::{self, Write},
    process::ExitCode,
};

const USAGE: &str = "electra-rc START_HZ STOP_HZ POINTS R_OHM C_F G_S [...]\nelectra-rc --noise START_HZ STOP_HZ POINTS R_OHM C_F G_S SERIES_K SHUNT_K [...]";

type Args = std::iter::Skip<env::Args>;
fn value(args: &mut Args) -> Result<f64, Box<dyn Error>> {
    Ok(args.next().ok_or(USAGE)?.parse()?)
}

// The compiler specializes this parser for signal stages and prepared noise stages.
fn stages<T>(
    args: &mut Args,
    finish: impl Fn(Stage, &mut Args) -> Result<T, Box<dyn Error>>,
) -> Result<Vec<T>, Box<dyn Error>> {
    let mut parts = Vec::new();
    while let Some(r) = args.next() {
        let cell = Stage::new(
            ElectricalImpedance::from_base(r.parse()?),
            Capacitance::from_base(value(args)?),
            ElectricConductance::from_base(value(args)?),
        )?;
        parts.push(finish(cell, args)?);
    }
    Ok(parts)
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let first = args.next().ok_or(USAGE)?;
    if first == "--help" {
        println!("{USAGE}");
        return Ok(());
    }
    let noise = first == "--noise";
    let start = if noise {
        value(&mut args)?
    } else {
        first.parse()?
    };
    let grid = LogSweep::new(
        Frequency::from_base(start),
        Frequency::from_base(value(&mut args)?),
        args.next().ok_or(USAGE)?.parse()?,
    )?;
    let mut out = io::BufWriter::new(io::stdout().lock());
    if noise {
        let parts = stages(&mut args, |stage, args| {
            Ok(ThermalStage::new(
                stage,
                ThermodynamicTemperature::from_base(value(args)?),
                ThermodynamicTemperature::from_base(value(args)?),
            )?)
        })?;
        output::noise(&mut out, grid, NoiseLadder::new(&parts)?)?;
    } else {
        let parts = stages(&mut args, |stage, _| Ok(stage))?;
        output::response(&mut out, grid, RcLadder::new(&parts)?)?;
    }
    out.flush()?;
    Ok(())
}
fn main() -> ExitCode {
    if let Err(e) = run() {
        eprintln!("{e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
