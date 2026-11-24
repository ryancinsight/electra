//! Electrical DC analysis for resistor networks and independent sources.
//! Use [`parse`] for the supported netlist dialect and [`Circuit::operating_point`] to solve.
mod netlist;
use aequitas::systems::si::dimensions;
pub use aequitas::systems::si::quantities::{
    Dimensionless, ElectricConductance, ElectricCurrent, ElectricPotential, ElectricalImpedance,
};
use electra_network::{Branch, solve};
/// Electrical convergence limits measured in amperes and dimensionless relative error.
/// ```compile_fail
/// use electra_circuit::{ElectricPotential, SolveOptions};
/// let _ = SolveOptions { absolute: ElectricPotential::from_base(1e-9), ..Default::default() };
/// ```
pub type SolveOptions = electra_network::SolveOptions<dimensions::ElectricCurrent>;
/// Electrical solver failure with a current-valued residual on nonconvergence.
pub type SolveError = electra_network::Error<dimensions::ElectricCurrent>;
pub use netlist::{ParseError, parse};

#[derive(Debug)]
enum Element {
    Resistor { siemens: ElectricConductance },
    Current { amperes: ElectricCurrent },
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
    fixed_volts: Vec<Option<ElectricPotential>>,
    injected_amps: Vec<ElectricCurrent>,
    branches: Vec<Branch<dimensions::ElectricConductance>>,
    parts: Vec<Part>,
}

/// Voltages and passive-sign branch currents in input order.
///
/// The compiler checks that current and voltage yield power.
///
/// ```
/// use aequitas::systems::si::quantities::Power;
/// use electra_circuit::parse;
/// let circuit = parse("V1 a 0 5\nR1 a 0 1000").unwrap();
/// let dc = circuit.operating_point(Default::default()).unwrap();
/// let dissipated: Power = dc.volts[1].1 * dc.amperes[1].1;
/// assert!((dissipated.into_base() - 0.025).abs() < 1e-12);
/// ```
/// ```compile_fail
/// use electra_circuit::{DcResult, ElectricCurrent};
/// fn wrong_probe(dc: &DcResult<'_>) -> ElectricCurrent { dc.volts[0].1 }
/// ```
#[derive(Debug)]
pub struct DcResult<'a> {
    /// Node name and potential relative to node zero (V).
    pub volts: Vec<(&'a str, ElectricPotential)>,
    /// Element name and current from first node to second (A).
    pub amperes: Vec<(&'a str, ElectricCurrent)>,
    /// Maximum absolute KCL imbalance at a free node (A).
    pub residual_amperes: ElectricCurrent,
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
