//! Command-line analysis selection and dimensioned convergence settings.
use electra_circuit::{Dimensionless, ElectricCurrent, SolveOptions};
use std::{ffi::OsString, path::PathBuf};

const USAGE: &str = "usage: electra {op|dc} FILE [--atol-amps A] [--rtol R] [--max-sweeps N]";

pub(super) enum Mode {
    OperatingPoint,
    Sweep,
}

pub(super) struct Request {
    pub mode: Mode,
    pub path: PathBuf,
    pub options: SolveOptions,
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
        while let Some(flag) = args.next() {
            let value = args.next().ok_or(USAGE)?;
            let value = value.to_str().ok_or(USAGE)?;
            match flag.to_str() {
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
        let defaults = SolveOptions::default();
        let options = SolveOptions {
            absolute: absolute.unwrap_or(defaults.absolute),
            relative: relative.unwrap_or(defaults.relative),
            max_sweeps: budget.unwrap_or(defaults.max_sweeps),
        };
        options.validate()?;
        Ok(Self {
            mode,
            path,
            options,
        })
    }
}
