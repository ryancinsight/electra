//! Command-line DC operating-point and transfer analysis.
mod cli;
use cli::{Mode, Request, Solver};
use electra_circuit::DcResult;
use std::{
    env,
    io::{self, Write},
    process::ExitCode,
};

fn write_point(out: &mut impl Write, result: &DcResult<'_>) -> io::Result<()> {
    result.write_probes(out)?;
    writeln!(
        out,
        "max KCL residual = {:.6e} A ({} sweeps)",
        result.residual_amperes.into_base(),
        result.sweeps
    )
}

fn write_mna(out: &mut impl Write, result: &electra_circuit::MnaPoint<'_>) -> io::Result<()> {
    result.probes.write_probes(out)?;
    writeln!(
        out,
        "max KCL residual = {:.6e} A; max source voltage residual = {:.6e} V ({} GMRES iterations)",
        result.probes.residual_amperes.into_base(),
        result.residual_volts.into_base(),
        result.iterations
    )
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1).peekable();
    if args.peek().is_some_and(|arg| arg == "ac-matrix") {
        args.next();
        let mut out = io::BufWriter::new(io::stdout().lock());
        cli::ac_matrix::run(args, &mut out)?;
        return Ok(out.flush()?);
    }
    let request = Request::parse(args)?;
    let circuit = electra_circuit::parse_file(request.path)?;
    let mut out = io::BufWriter::new(io::stdout().lock());
    match (
        request.mode,
        request.settings.select(circuit.requires_mna())?,
    ) {
        (Mode::OperatingPoint, Solver::Reduced(options)) => {
            write_point(&mut out, &circuit.operating_point(options)?)?
        }
        (Mode::Sweep, Solver::Reduced(options)) => {
            let mut sweep = circuit.dc_sweep(options)?;
            let name = sweep.source_name();
            while let Some(sample) = sweep.next_point()? {
                writeln!(out, "{name} = {}", sample.excitation)?;
                write_point(&mut out, sample.result)?;
            }
        }
        (Mode::OperatingPoint, Solver::Mna(options)) => {
            write_mna(&mut out, &circuit.operating_point_mna(options)?)?
        }
        (Mode::Sweep, Solver::Mna(options)) => {
            let mut sweep = circuit.dc_sweep_mna(options)?;
            let name = sweep.source_name();
            while let Some(sample) = sweep.next_point()? {
                writeln!(out, "{name} = {}", sample.excitation)?;
                write_mna(&mut out, sample.result)?;
            }
        }
    }
    out.flush()?;
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("electra: {err}");
            ExitCode::FAILURE
        }
    }
}
