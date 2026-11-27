//! Series RC, RL and RLC transients under a constant voltage over each step.
use crate::{ElectricCurrent, ElectricPotential, ElectricalImpedance};
pub use aequitas::systems::si::quantities::{Capacitance, Inductance, Time};
use std::fmt;
mod rlc;
mod step;
pub use rlc::{AdaptiveOptions, RlcCheckpoint, RlcState, SegmentResult, SeriesRlc};

/// Invalid input or a result outside the finite representable range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepError {
    /// Resistance, capacitance, inductance or the derived time constant is invalid.
    Component,
    /// Elapsed time must be finite and nonnegative.
    Interval,
    /// The initial state, excitation or resulting state is outside the numeric range.
    State,
}

impl fmt::Display for StepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Component => "invalid passive component value or time constant",
            Self::Interval => "expected finite nonnegative step duration",
            Self::State => "transient state or drive exceeds numeric range",
        })
    }
}
impl std::error::Error for StepError {}

/// Series RC model. The state is the voltage across the capacitor.
pub struct SeriesRc {
    tau: Time,
}

impl SeriesRc {
    /// Construct a model with time constant RC.
    pub fn new(r: ElectricalImpedance, c: Capacitance) -> Result<Self, StepError> {
        step::positive(r.into_base())?;
        step::positive(c.into_base())?;
        let tau: Time = r * c;
        step::positive(tau.into_base())?;
        Ok(Self { tau })
    }

    /// Advance the supplied capacitor voltage by one backward Euler step.
    pub fn advance(
        &self,
        state: ElectricPotential,
        drive: ElectricPotential,
        dt: Time,
    ) -> Result<ElectricPotential, StepError> {
        step::advance(state.into_base(), drive.into_base(), self.tau, dt)
            .map(ElectricPotential::from_base)
    }
}

/// Series RL model. Positive current flows from the positive drive terminal.
pub struct SeriesRl {
    tau: Time,
    resistance: ElectricalImpedance,
}

impl SeriesRl {
    /// Construct a model with time constant L/R.
    pub fn new(r: ElectricalImpedance, l: Inductance) -> Result<Self, StepError> {
        step::positive(r.into_base())?;
        step::positive(l.into_base())?;
        let tau: Time = l / r;
        step::positive(tau.into_base())?;
        Ok(Self { tau, resistance: r })
    }

    /// Advance the supplied inductor current by one backward Euler step.
    pub fn advance(
        &self,
        state: ElectricCurrent,
        drive: ElectricPotential,
        dt: Time,
    ) -> Result<ElectricCurrent, StepError> {
        let target: ElectricCurrent = drive / self.resistance;
        step::advance(state.into_base(), target.into_base(), self.tau, dt)
            .map(ElectricCurrent::from_base)
    }
}
