//! Electrical DC analysis for resistor networks and independent sources.
//! Use [`parse`] for the supported netlist dialect and [`Circuit::operating_point`] to solve.
mod netlist;
use electra_network::{Branch, solve};
pub use electra_network::{Error as SolveError, SolveOptions};
pub use netlist::{ParseError, parse};

#[derive(Debug)]
enum Element {
    Resistor { siemens: f64 },
    Current { amperes: f64 },
    Voltage { prescribed: usize },
}
#[derive(Debug)]
struct Part {
    name: String,
    positive: usize,
    negative: usize,
    element: Element,
}

/// A checked, named circuit. Node zero is the reference.
#[derive(Debug)]
pub struct Circuit {
    names: Vec<String>,
    fixed_volts: Vec<Option<f64>>,
    injected_amps: Vec<f64>,
    branches: Vec<Branch<f64>>,
    parts: Vec<Part>,
}

/// Voltages and passive-sign branch currents in input order.
#[derive(Debug)]
pub struct DcResult<'a> {
    /// Node name and potential relative to node zero (V).
    pub volts: Vec<(&'a str, f64)>,
    /// Element name and current from first node to second (A).
    pub amperes: Vec<(&'a str, f64)>,
    /// Maximum absolute KCL imbalance at a free node (A).
    pub residual_amperes: f64,
    /// Number of relaxation sweeps required.
    pub sweeps: usize,
}

impl Circuit {
    /// Solve the DC operating point. Absolute tolerance is in amperes.
    pub fn operating_point(&self, options: SolveOptions) -> Result<DcResult<'_>, SolveError> {
        let point = solve(
            &self.fixed_volts,
            &self.injected_amps,
            &self.branches,
            options,
        )?;
        let amperes = self
            .parts
            .iter()
            .map(|part| {
                let current = match part.element {
                    Element::Resistor { siemens } => {
                        siemens
                            * (point.potentials[part.positive] - point.potentials[part.negative])
                    }
                    Element::Current { amperes } => amperes,
                    Element::Voltage { prescribed } if prescribed == part.positive => {
                        -point.reactions[prescribed]
                    }
                    Element::Voltage { prescribed } => point.reactions[prescribed],
                };
                (part.name.as_str(), current)
            })
            .collect();
        Ok(DcResult {
            volts: self
                .names
                .iter()
                .map(String::as_str)
                .zip(point.potentials)
                .collect(),
            amperes,
            residual_amperes: point.max_residual,
            sweeps: point.sweeps,
        })
    }
}
