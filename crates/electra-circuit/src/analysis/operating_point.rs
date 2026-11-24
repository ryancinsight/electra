use crate::{Circuit, ElectricCurrent, ElectricPotential, Element, SolveError, SolveOptions};
use aequitas::systems::si::dimensions;
use electra_network::{OperatingPoint, solve};

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
    // Omit a swept current source before summing. Subtracting its nominal value
    // from an accumulated RHS can erase smaller independent injections.
    pub(super) fn injections(&self, omit: Option<usize>) -> Vec<ElectricCurrent> {
        let mut injected = vec![ElectricCurrent::default(); self.names.len()];
        for (index, part) in self.parts.iter().enumerate() {
            if let Element::Current { amperes } = part.element
                && Some(index) != omit
            {
                injected[part.positive] -= amperes;
                injected[part.negative] += amperes;
            }
        }
        injected
    }

    /// Solve the nominal DC operating point. Absolute tolerance is in amperes.
    pub fn operating_point(&self, options: SolveOptions) -> Result<DcResult<'_>, SolveError> {
        let point = solve(
            &self.fixed_volts,
            &self.injections(None),
            &self.branches,
            options,
        )?;
        let mut result = DcResult::new(self);
        result.update(self, &point, None);
        Ok(result)
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

    pub(super) fn update(
        &mut self,
        circuit: &Circuit,
        point: &OperatingPoint<dimensions::ElectricPotential, dimensions::ElectricCurrent>,
        current_override: Option<(usize, ElectricCurrent)>,
    ) {
        for ((_, out), &v) in self.volts.iter_mut().zip(&point.potentials) {
            *out = v;
        }
        for ((_, out), part) in self.amperes.iter_mut().zip(&circuit.parts) {
            *out = match part.element {
                Element::Resistor { branch } => {
                    circuit.branches[branch].conductance
                        * (point.potentials[part.positive] - point.potentials[part.negative])
                }
                Element::Current { amperes } => amperes,
                Element::Voltage { prescribed } if prescribed == part.positive => {
                    -point.reactions[prescribed]
                }
                Element::Voltage { prescribed } => point.reactions[prescribed],
            };
        }
        if let Some((index, value)) = current_override {
            self.amperes[index].1 = value;
        }
        self.residual_amperes = point.max_residual;
        self.sweeps = point.sweeps;
    }
}
