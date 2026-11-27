//! Startup steps for a half-wave capacitor-input rectifier and an ideal synchronous buck.
use super::{
    Capacitance, ElectricCurrent, ElectricPotential, ElectricalImpedance, Inductance, RlcState,
    Time,
};
mod buck;
mod rectifier;
pub use buck::SynchronousBuck;
pub use rectifier::HalfWaveRectifier;

/// Distinct failure classes for a trial step. Callers retain the previous state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerError {
    /// Nonfinite or invalid component, drive, state or Newton tolerance.
    Input,
    /// The requested positive interval is zero or too small for the step coefficients.
    TimestepExhausted,
    /// The linearized equation has a zero pivot.
    Singular,
    /// Newton iterations exhausted their supplied budget.
    NewtonLimit,
    /// A coefficient or state exceeds finite arithmetic.
    Range,
}
impl std::fmt::Display for PowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Input => "invalid power-stage input",
            Self::TimestepExhausted => "power-stage timestep exhausted",
            Self::Singular => "singular power-stage equation",
            Self::NewtonLimit => "power-stage Newton iteration limit",
            Self::Range => "power-stage arithmetic exceeds numeric range",
        })
    }
}
impl std::error::Error for PowerError {}
fn positive(value: f64) -> bool {
    value.is_finite() && value > 0.
}
fn interval(dt: Time) -> Result<f64, PowerError> {
    let dt = dt.into_base();
    if dt == 0. {
        Err(PowerError::TimestepExhausted)
    } else if positive(dt) {
        Ok(dt)
    } else {
        Err(PowerError::Input)
    }
}
