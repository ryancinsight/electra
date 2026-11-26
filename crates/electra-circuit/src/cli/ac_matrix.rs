//! Deterministic sparse AC matrix export.
use electra_ac::{Complex64, Frequency};
use std::{error::Error, ffi::OsString, fs, io::Write, path::PathBuf};

pub(crate) fn run(
    mut args: impl Iterator<Item = OsString>,
    out: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let usage = "usage: electra ac-matrix FILE FREQUENCY_HZ";
    let path = PathBuf::from(args.next().ok_or(usage)?);
    let hz = args
        .next()
        .and_then(|v| v.into_string().ok())
        .ok_or(usage)?
        .parse()?;
    if args.next().is_some() {
        return Err(usage.into());
    }
    let text = fs::read_to_string(&path)?;
    let located = |e| format!("{}: {e}", path.display());
    let deck = electra_circuit::parse_ac(&text).map_err(located)?;
    drop(text);
    let system = deck.assemble(Frequency::from_base(hz)).map_err(located)?;
    writeln!(out, "kind,row,column,label,real,imag")?;
    for (i, (name, fixed)) in deck.nodes().iter().zip(deck.boundary()).enumerate() {
        let escaped;
        let name = if name.contains('"') {
            escaped = name.replace('"', "\"\"");
            &escaped
        } else {
            name
        };
        match fixed {
            Some(v) => writeln!(
                out,
                "fixed,{i},,\"{name}\",{},{}",
                v.as_base().re,
                v.as_base().im
            )?,
            None => writeln!(out, "free,{i},,\"{name}\",,")?,
        }
    }
    for (i, j, y) in system.coefficients() {
        let y = y.into_base();
        writeln!(out, "Y,{i},{j},,{},{}", y.re, y.im)?;
    }
    for (i, b) in system.currents().map(|b| b.into_base()).enumerate() {
        if b != Complex64::default() {
            writeln!(out, "rhs,{i},,,{},{}", b.re, b.im)?;
        }
    }
    Ok(())
}
