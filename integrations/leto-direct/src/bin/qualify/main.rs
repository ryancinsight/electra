//! Run the exact-reference suites against Leto's dense and sparse dispatch paths.
#![forbid(unsafe_code)]
mod dispatch;
mod profile;
use dispatch::Dispatch;
use electra_circuit::{
    Circuit, Dimensionless, ElectricCurrent, ElectricPotential, MnaOptions, MnaPoint,
    direct::DirectFactor,
};
use electra_leto_direct::PivotedLu;
use std::{
    env,
    error::Error,
    io::{self, Write},
};

fn point(out: &mut impl Write, p: &MnaPoint<'_>) -> io::Result<()> {
    p.probes.write_probes(out)?;
    writeln!(
        out,
        "max KCL residual = {:.6e} A; max source voltage residual = {:.6e} V",
        p.probes.residual_amperes.into_base(),
        p.residual_volts.into_base()
    )
}
fn run<F: DirectFactor>(c: &Circuit, mode: &str, limits: MnaOptions) -> Result<(), Box<dyn Error>> {
    let mut out = io::BufWriter::new(io::stdout().lock());
    match mode {
        "profile" => return profile::run::<F>(c, limits),
        "op" => point(&mut out, &c.operating_point_direct::<F>(limits)?)?,
        "dc" => {
            let mut sweep = c.dc_sweep_direct::<F>(limits)?;
            let source = sweep.source_name();
            while let Some(sample) = sweep.next_point()? {
                writeln!(out, "{source} = {}", sample.excitation)?;
                point(&mut out, sample.result)?;
            }
        }
        _ => return Err("expected op, dc or profile".into()),
    }
    out.flush()?;
    Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os().skip(1);
    let mode = args.next().ok_or("expected op, dc or profile")?;
    let path = args.next().ok_or("expected netlist path")?;
    let mut limits = MnaOptions::default();
    let mut provider = String::from("direct");
    while let Some(key) = args.next() {
        let value = args
            .next()
            .and_then(|x| x.into_string().ok())
            .ok_or("missing option value")?;
        match key.to_str() {
            Some("--solver") => provider = value,
            Some("--atol-amps") => {
                limits.current_absolute = ElectricCurrent::from_base(value.parse()?)
            }
            Some("--atol-volts") => {
                limits.voltage_absolute = ElectricPotential::from_base(value.parse()?)
            }
            Some("--rtol") => limits.relative = Dimensionless::from_base(value.parse()?),
            _ => return Err("unknown qualification option".into()),
        }
    }
    let c = electra_circuit::parse_file(path)?;
    let mode = mode.to_str().ok_or("expected op, dc or profile")?;
    match provider.as_str() {
        "direct" => run::<PivotedLu>(&c, mode, limits),
        "audit" => run::<dispatch::Audit>(&c, mode, limits),
        "dispatch" => run::<Dispatch<false>>(&c, mode, limits),
        "sparse-first" => run::<Dispatch<true>>(&c, mode, limits),
        _ => Err("expected direct, dispatch, sparse-first or audit".into()),
    }
}
