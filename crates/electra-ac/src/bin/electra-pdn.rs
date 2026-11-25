//! Stream complex driving-point impedance for a parallel capacitor bank.
use electra_ac::pdn::{CapacitorBank, PortImpedance, SeriesCapacitor};
use electra_ac::{Capacitance, ElectricalImpedance, Frequency, Inductance, LogSweep};
use std::{
    env,
    error::Error,
    io::{self, Write},
    process::ExitCode,
};

const USAGE: &str = "electra-pdn START_HZ STOP_HZ POINTS C_F ESR_OHM ESL_H [C_F ESR_OHM ESL_H ...]";

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let start = args.next().ok_or(USAGE)?;
    if start == "--help" {
        println!("{USAGE}");
        return Ok(());
    }
    let grid = LogSweep::new(
        Frequency::from_base(start.parse()?),
        Frequency::from_base(args.next().ok_or(USAGE)?.parse()?),
        args.next().ok_or(USAGE)?.parse()?,
    )?;
    let mut branches = Vec::new();
    while let Some(c) = args.next() {
        branches.push(SeriesCapacitor::new(
            Capacitance::from_base(c.parse()?),
            ElectricalImpedance::from_base(args.next().ok_or(USAGE)?.parse()?),
            Inductance::from_base(args.next().ok_or(USAGE)?.parse()?),
        )?);
    }
    if branches.is_empty() {
        return Err(USAGE.into());
    }
    let bank = CapacitorBank::new(&branches);
    let mut output = io::BufWriter::new(io::stdout().lock());
    writeln!(
        output,
        "frequency_Hz,port,z_re_ohm,z_im_ohm,z_magnitude_ohm"
    )?;
    for sample in grid {
        let frequency = sample?;
        let f = frequency.into_base();
        let z = bank.at(frequency)?;
        match z {
            PortImpedance::Open => writeln!(output, "{f},open,,,inf")?,
            PortImpedance::Finite(value) => {
                let value = value.into_base();
                writeln!(
                    output,
                    "{f},finite,{},{},{}",
                    value.re,
                    value.im,
                    z.magnitude().into_base()
                )?;
            }
        }
    }
    output.flush()?;
    Ok(())
}

fn main() -> ExitCode {
    if let Err(e) = run() {
        eprintln!("{e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
