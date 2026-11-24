//! Command-line DC operating-point simulator.
use std::{env, fs, process::ExitCode};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = env::args_os().skip(1);
    let (Some(mode), Some(path), None) = (arguments.next(), arguments.next(), arguments.next())
    else {
        return Err("usage: electra op FILE".into());
    };
    if mode != "op" {
        return Err("usage: electra op FILE".into());
    }
    let circuit = electra_circuit::parse(&fs::read_to_string(path)?)?;
    let result = circuit.operating_point(Default::default())?;
    for (name, v) in result.volts {
        println!("V({name}) = {v:.12e} V");
    }
    for (name, i) in result.amperes {
        println!("I({name}) = {i:.12e} A");
    }
    println!(
        "max KCL residual = {:.6e} A ({} sweeps)",
        result.residual_amperes, result.sweeps
    );
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
