mod error;
pub use error::SweepError;
mod grid;
mod plan;

use crate::{Circuit, DcResult, ElectricCurrent, ElectricPotential, SolveOptions};
use aequitas::systems::si::dimensions;
use electra_network::PreparedNetwork;
use grid::Grid;
pub use grid::LinearSweep;
pub(crate) use plan::SweepPlan;
use std::fmt;

/// Typed input range for an independent source.
/// ```compile_fail
/// use electra_circuit::{ElectricPotential, LinearSweep, SweepRange};
/// let volts = ElectricPotential::from_base(1.0);
/// let _ = SweepRange::Current(LinearSweep { start: volts, stop: volts, step: volts });
/// ```
pub enum SweepRange {
    /// Range in volts.
    Voltage(LinearSweep<dimensions::ElectricPotential>),
    /// Range in amperes.
    Current(LinearSweep<dimensions::ElectricCurrent>),
}
/// Applied excitation, measured from the positive terminal to the negative terminal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SourceValue {
    /// Voltage-source excitation in volts.
    Voltage(ElectricPotential),
    /// Current-source excitation in amperes.
    Current(ElectricCurrent),
}
impl fmt::Display for SourceValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Voltage(q) => write!(f, "{:.12e} V", q.as_base()),
            Self::Current(q) => write!(f, "{:.12e} A", q.as_base()),
        }
    }
}

/// Borrowed readout of one operating point. Copy requested probes before advancing.
pub struct DcSample<'point, 'circuit> {
    /// Applied excitation in source orientation.
    pub excitation: SourceValue,
    /// Voltages, currents and convergence diagnostics in reusable storage.
    pub result: &'point DcResult<'circuit>,
}

/// Streaming transfer analysis. Storage depends on circuit size and is reused at every point.
pub struct DcSweep<'a> {
    circuit: &'a Circuit,
    network: PreparedNetwork<
        'a,
        dimensions::ElectricPotential,
        dimensions::ElectricConductance,
        dimensions::ElectricCurrent,
    >,
    fixed: Vec<Option<ElectricPotential>>,
    injections: Vec<ElectricCurrent>,
    baseline: [ElectricCurrent; 2],
    plan: SweepPlan,
    output: DcResult<'a>,
    options: SolveOptions,
    finished: bool,
}

impl Circuit {
    /// Prepare the linear sweep declared by a .dc card.
    pub fn dc_sweep(&self, options: SolveOptions) -> Result<DcSweep<'_>, SweepError> {
        DcSweep::new(
            self,
            self.sweep.ok_or(SweepError::MissingDirective)?,
            options,
        )
    }

    /// Sweep a named independent source using an explicitly dimensioned range.
    pub fn sweep_source(
        &self,
        name: &str,
        range: SweepRange,
        options: SolveOptions,
    ) -> Result<DcSweep<'_>, SweepError> {
        DcSweep::new(self, self.plan(name, range)?, options)
    }
}

impl<'a> DcSweep<'a> {
    fn new(
        circuit: &'a Circuit,
        plan: SweepPlan,
        options: SolveOptions,
    ) -> Result<Self, SweepError> {
        let part = &circuit.parts[plan.source];
        let injections = circuit.injections(Some(plan.source));
        let baseline = [injections[part.positive], injections[part.negative]];
        Ok(Self {
            circuit,
            network: PreparedNetwork::new(&circuit.fixed_volts, &circuit.branches)
                .map_err(SweepError::Network)?,
            fixed: circuit.fixed_volts.clone(),
            baseline,
            injections,
            plan,
            output: DcResult::new(circuit),
            options,
            finished: false,
        })
    }
    /// Name of the swept source in its original spelling.
    pub fn source_name(&self) -> &'a str {
        &self.circuit.parts[self.plan.source].name
    }

    /// Advance one point. Exhaustion and an error both terminate the stream.
    pub fn next_point(&mut self) -> Result<Option<DcSample<'_, 'a>>, SweepError> {
        if self.finished {
            return Ok(None);
        }
        self.finished = true;
        let Some(excitation) = self.plan.axis.next()? else {
            return Ok(None);
        };
        let part = &self.circuit.parts[self.plan.source];
        let mut current_override = None;
        match excitation {
            SourceValue::Current(value) => {
                self.injections[part.positive] = self.baseline[0] - value;
                self.injections[part.negative] = self.baseline[1] + value;
                current_override = Some((self.plan.source, value));
            }
            SourceValue::Voltage(value) => {
                let (node, voltage) =
                    crate::prescribed_voltage(part.positive, part.negative, value);
                self.fixed[node] = Some(voltage);
            }
        }
        let point = self
            .network
            .solve(&self.fixed, &self.injections, self.options)
            .map_err(|err| SweepError::Point(excitation, err))?;
        self.output.update(self.circuit, point, current_override);
        self.finished = false;
        Ok(Some(DcSample {
            excitation,
            result: &self.output,
        }))
    }
}
