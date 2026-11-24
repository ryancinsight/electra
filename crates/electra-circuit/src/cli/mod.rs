//! Command-line analysis selection and dimensioned convergence settings.
use electra_circuit::{
    Dimensionless, ElectricCurrent, ElectricPotential, MnaOptions, SolveOptions,
};
use std::{ffi::OsString, path::PathBuf};

const USAGE: &str = "usage: electra {op|dc} FILE [--solver reduced|mna] [--atol-amps A] [--rtol R] [--max-sweeps N (reduced)] [--atol-volts V] [--max-iterations N (mna)]";

pub(super) enum Mode {
    OperatingPoint,
    Sweep,
}

pub(super) enum Solver {
    Reduced(SolveOptions),
    Mna(MnaOptions),
}

pub(super) struct Request {
    pub mode: Mode,
    pub path: PathBuf,
    pub settings: Settings,
}

impl Request {
    pub fn parse(
        mut args: impl Iterator<Item = OsString>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mode = match args.next().as_deref().and_then(|s| s.to_str()) {
            Some("op") => Mode::OperatingPoint,
            Some("dc") => Mode::Sweep,
            _ => return Err(USAGE.into()),
        };
        let path = args.next().ok_or(USAGE)?.into();
        let (mut absolute, mut relative, mut budget) = (None, None, None);
        let (mut solver, mut volts, mut iterations) = (None, None, None);
        while let Some(flag) = args.next() {
            let value = args.next().ok_or(USAGE)?;
            let value = value.to_str().ok_or(USAGE)?;
            match flag.to_str() {
                Some("--solver") if solver.is_none() => {
                    solver = Some(match value {
                        "reduced" => false,
                        "mna" => true,
                        _ => return Err(USAGE.into()),
                    })
                }
                Some("--atol-volts") if volts.is_none() => {
                    volts = Some(ElectricPotential::from_base(value.parse()?))
                }
                Some("--max-iterations") if iterations.is_none() => {
                    iterations = Some(value.parse()?)
                }
                Some("--atol-amps") if absolute.is_none() => {
                    absolute = Some(ElectricCurrent::from_base(value.parse()?))
                }
                Some("--rtol") if relative.is_none() => {
                    relative = Some(Dimensionless::from_base(value.parse()?))
                }
                Some("--max-sweeps") if budget.is_none() => budget = Some(value.parse()?),
                _ => return Err(USAGE.into()),
            }
        }
        Ok(Self {
            mode,
            path,
            settings: Settings {
                solver,
                absolute,
                relative,
                budget,
                volts,
                iterations,
            },
        })
    }
}

pub(super) struct Settings {
    solver: Option<bool>,
    absolute: Option<ElectricCurrent>,
    relative: Option<Dimensionless>,
    budget: Option<usize>,
    volts: Option<ElectricPotential>,
    iterations: Option<usize>,
}
impl Settings {
    pub fn select(self, controlled: bool) -> Result<Solver, Box<dyn std::error::Error>> {
        let Self {
            solver,
            absolute,
            relative,
            budget,
            volts,
            iterations,
        } = self;
        if controlled && solver == Some(false) {
            return Err("controlled sources require the MNA solver".into());
        }
        let defaults = SolveOptions::default();
        let options = SolveOptions {
            absolute: absolute.unwrap_or(defaults.absolute),
            relative: relative.unwrap_or(defaults.relative),
            max_sweeps: budget.unwrap_or(defaults.max_sweeps),
        };
        let solver = if solver.unwrap_or(controlled) {
            if budget.is_some() {
                return Err(
                    "--max-sweeps applies to the reduced solver; use --max-iterations for MNA"
                        .into(),
                );
            }
            let defaults = MnaOptions::default();
            Solver::Mna(MnaOptions {
                current_absolute: options.absolute,
                relative: options.relative,
                voltage_absolute: volts.unwrap_or(defaults.voltage_absolute),
                max_iterations: iterations.unwrap_or(defaults.max_iterations),
            })
        } else {
            if volts.is_some() || iterations.is_some() {
                return Err("--atol-volts and --max-iterations require --solver mna".into());
            }
            options.validate()?;
            Solver::Reduced(options)
        };
        Ok(solver)
    }
}
