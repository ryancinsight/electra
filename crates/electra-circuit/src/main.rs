//! Command-line DC operating-point and transfer analysis.
mod cli;
use cli::{Mode, Request};
use electra_circuit::DcResult;
use std::{
    env,
    io::{self, Write},
    process::ExitCode,
};

fn write_point(out: &mut impl Write, result: &DcResult<'_>) -> io::Result<()> {
    for &(name, v) in &result.volts {
        writeln!(out, "V({name}) = {:.12e} V", v.into_base())?;
    }
    for &(name, i) in &result.amperes {
        writeln!(out, "I({name}) = {:.12e} A", i.into_base())?;
    }
    writeln!(
        out,
        "max KCL residual = {:.6e} A ({} sweeps)",
        result.residual_amperes.into_base(),
        result.sweeps
    )
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let request = Request::parse(env::args_os().skip(1))?;
    let circuit = electra_circuit::parse_file(request.path)?;
    let mut out = io::BufWriter::new(io::stdout().lock());
    match request.mode {
        Mode::OperatingPoint => write_point(&mut out, &circuit.operating_point(request.options)?)?,
        Mode::Sweep => {
            let mut sweep = circuit.dc_sweep(request.options)?;
            let name = sweep.source_name();
            while let Some(sample) = sweep.next_point()? {
                writeln!(out, "{name} = {}", sample.excitation)?;
                write_point(&mut out, sample.result)?;
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
