mod error;
pub use error::SweepError;
mod grid;
mod plan;

use super::dc::Session;
use crate::{Circuit, DcResult, ElectricCurrent, ElectricPotential, SolveOptions};
use aequitas::systems::si::dimensions;
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
    session: Session<'a>,
    name: &'a str,
    plan: SweepPlan,
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
        Ok(Self {
            session: Session::new(circuit, Some(plan.source)).map_err(SweepError::Network)?,
            name: &circuit.parts[plan.source].name,
            plan,
            options,
            finished: false,
        })
    }
    /// Name of the swept source in its original spelling.
    pub fn source_name(&self) -> &'a str {
        self.name
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
        self.session
            .run(Some((self.plan.source, excitation)), self.options)
            .map_err(|err| SweepError::Point(excitation, err))?;
        self.finished = false;
        Ok(Some(DcSample {
            excitation,
            result: &self.session.output,
        }))
    }
}
