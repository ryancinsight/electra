use super::dc::Session;
use crate::{Circuit, ElectricCurrent, ElectricPotential, SolveError, SolveOptions};
use std::io::{self, Write};

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
    /// Maximum absolute checked KCL imbalance in the original circuit (A).
    pub residual_amperes: ElectricCurrent,
    /// Number of relaxation sweeps required.
    pub sweeps: usize,
}

impl Circuit {
    /// Solve independent-source DC bias by reduction, with passive-sign branch currents.
    /// Circuits for which [`Circuit::requires_mna`] is true use [`Circuit::operating_point_mna`].
    pub fn operating_point(&self, options: SolveOptions) -> Result<DcResult<'_>, SolveError> {
        options.validate()?;
        let mut session = Session::new(self, None)?;
        session.run(None, options)?;
        Ok(session.output)
    }
}

impl<'a> DcResult<'a> {
    pub(super) fn new(circuit: &'a Circuit) -> Self {
        Self {
            volts: circuit
                .names
                .iter()
                .map(|name| (name.as_str(), ElectricPotential::default()))
                .collect(),
            amperes: circuit
                .parts
                .iter()
                .map(|part| (part.name.as_str(), ElectricCurrent::default()))
                .collect(),
            sweeps: 0,
            residual_amperes: ElectricCurrent::default(),
        }
    }
}

impl DcResult<'_> {
    /// Write all named potentials and signed branch currents in SI units and deck order.
    pub fn write_probes(&self, out: &mut impl Write) -> io::Result<()> {
        for &(name, v) in &self.volts {
            writeln!(out, "V({name}) = {:.12e} V", v.into_base())?;
        }
        for &(name, i) in &self.amperes {
            writeln!(out, "I({name}) = {:.12e} A", i.into_base())?;
        }
        Ok(())
    }
}
