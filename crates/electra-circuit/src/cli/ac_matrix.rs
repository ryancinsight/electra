//! Deterministic sparse AC matrix export.
use electra_ac::{Complex64, Frequency, PhasorSum};
use std::{collections::BTreeMap, error::Error, ffi::OsString, fs, io::Write, path::PathBuf};

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
    let mut matrix = BTreeMap::<(usize, usize), PhasorSum>::new();
    let mut rhs = vec![PhasorSum::default(); deck.nodes().len()];
    deck.stamp(
        Frequency::from_base(hz),
        |i, j, y| matrix.entry((i, j)).or_default().add(y.into_base()),
        |i, b| rhs[i].add(b.into_base()),
    )
    .map_err(located)?;
    if matrix
        .values()
        .chain(&rhs)
        .map(|sum| sum.total())
        .any(|z| !z.re.is_finite() || !z.im.is_finite())
    {
        return Err("AC coefficient accumulation overflow".into());
    }
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
    for ((i, j), sum) in matrix {
        let y = sum.total();
        if y != Complex64::default() {
            writeln!(out, "Y,{i},{j},,{},{}", y.re, y.im)?;
        }
    }
    for (i, b) in rhs.into_iter().map(PhasorSum::total).enumerate() {
        if b != Complex64::default() {
            writeln!(out, "rhs,{i},,,{},{}", b.re, b.im)?;
        }
    }
    Ok(())
}
