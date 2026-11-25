//! Stream a passive RC ladder's gain, phase and output impedance as CSV.
use aequitas::systems::si::units::Degree;
use electra_ac::{
    Capacitance, ElectricConductance, ElectricalImpedance, Frequency, LogSweep, RcLadder, Stage,
};
use std::{
    env,
    error::Error,
    io::{self, Write},
};

const USAGE: &str = "electra-rc START_HZ STOP_HZ POINTS R_OHM C_F G_S [R_OHM C_F G_S ...]";

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let first = args.next().ok_or(USAGE)?;
    if first == "--help" {
        println!("{USAGE}");
        return Ok(());
    }
    let start = Frequency::from_base(first.parse()?);
    let stop = Frequency::from_base(args.next().ok_or(USAGE)?.parse()?);
    let count = args.next().ok_or(USAGE)?.parse()?;
    let grid = LogSweep::new(start, stop, count)?;
    let mut stages = Vec::new();
    while let Some(r) = args.next() {
        let series = ElectricalImpedance::from_base(r.parse()?);
        let cap = Capacitance::from_base(args.next().ok_or(USAGE)?.parse()?);
        let load = ElectricConductance::from_base(args.next().ok_or(USAGE)?.parse()?);
        stages.push(Stage::new(series, cap, load)?);
    }
    let model = RcLadder::new(&stages)?;
    let mut out = io::BufWriter::new(io::stdout().lock());
    writeln!(
        out,
        "frequency_Hz,gain_re,gain_im,gain_magnitude,phase_deg,zout_re_ohm,zout_im_ohm"
    )?;
    for f in grid {
        let point = model.at(f?)?;
        let gain = point.gain.into_base();
        let z = point.output_impedance.into_base();
        let phase = point
            .phase()
            .ok_or("undefined output phase")?
            .in_unit::<Degree>();
        writeln!(
            out,
            "{},{},{},{},{},{},{}",
            point.frequency.into_base(),
            gain.re,
            gain.im,
            point.magnitude().into_base(),
            phase,
            z.re,
            z.im
        )?;
    }
    out.flush()?;
    Ok(())
}
fn main() -> std::process::ExitCode {
    if let Err(e) = run() {
        eprintln!("{e}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}
