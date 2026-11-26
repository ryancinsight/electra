//! Solve one flat AC deck and emit original-node phasor voltages.
use electra_ac::Frequency;
use electra_circuit::parse_ac;
use electra_leto_direct::ac;
use std::{
    error::Error,
    io::{self, Write},
    path::PathBuf,
};

fn main() -> Result<(), Box<dyn Error>> {
    let usage = "usage: electra-ac-solve FILE FREQUENCY_HZ";
    let mut args = std::env::args_os().skip(1);
    let path = PathBuf::from(args.next().ok_or(usage)?);
    let hz = args
        .next()
        .and_then(|x| x.into_string().ok())
        .ok_or(usage)?
        .parse()?;
    if args.next().is_some() {
        return Err(usage.into());
    }
    let deck = parse_ac(&std::fs::read_to_string(path)?)?;
    let point = ac::solve(&deck, Frequency::from_base(hz))?;
    let mut out = io::BufWriter::new(io::stdout().lock());
    writeln!(out, "node,real_V,imag_V")?;
    for (name, v) in deck.nodes().iter().zip(point.volts) {
        let z = v.into_base();
        writeln!(out, "\"{}\",{},{}", name.replace('"', "\"\""), z.re, z.im)?;
    }
    out.flush()?;
    eprintln!("max KCL residual = {} A", point.residual.into_base());
    Ok(())
}
