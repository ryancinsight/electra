//! Implicit midpoint integration of a voltage-driven series RLC branch.
use super::{
    Capacitance, ElectricCurrent, ElectricPotential, ElectricalImpedance, Inductance, StepError,
    Time, step,
};

/// Instantaneous capacitor voltage and series current directed into its positive plate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RlcState {
    /// Voltage across the capacitor.
    pub voltage: ElectricPotential,
    /// Current through the inductor.
    pub current: ElectricCurrent,
}

/// Series resistor, inductor and capacitor driven by a prescribed voltage.
pub struct SeriesRlc {
    resistance: ElectricalImpedance,
    inductance: Inductance,
    capacitance: Capacitance,
}

impl SeriesRlc {
    /// Construct a passive series branch. Zero resistance models a lossless LC circuit.
    pub fn new(
        resistance: ElectricalImpedance,
        inductance: Inductance,
        capacitance: Capacitance,
    ) -> Result<Self, StepError> {
        if !resistance.into_base().is_finite() || resistance.into_base() < 0. {
            return Err(StepError::Component);
        }
        step::positive(inductance.into_base())?;
        step::positive(capacitance.into_base())?;
        Ok(Self {
            resistance,
            inductance,
            capacitance,
        })
    }

    /// Advance both states with the voltage held constant over one interval.
    /// The step uses a closed two-equation solve and allocates no waveform or matrix.
    pub fn advance(
        &self,
        state: RlcState,
        drive: ElectricPotential,
        dt: Time,
    ) -> Result<RlcState, StepError> {
        let dt = step::interval(dt)?;
        let voltage = state.voltage.into_base();
        let current = state.current.into_base();
        let source = drive.into_base();
        if !voltage.is_finite() || !current.is_finite() || !source.is_finite() {
            return Err(StepError::State);
        }
        if dt == 0. {
            return Ok(state);
        }
        // h = dt/2. At midpoint the capacitor voltage is v0 + (h/C) i_mid.
        let a = (dt / self.inductance.into_base()) * 0.5;
        let b = (dt / self.capacitance.into_base()) * 0.5;
        let denominator = a.mul_add(self.resistance.into_base(), a.mul_add(b, 1.));
        if !denominator.is_finite() {
            return Err(StepError::State);
        }
        let midpoint = a.mul_add(source - voltage, current) / denominator;
        let next_voltage = b.mul_add(midpoint, b.mul_add(midpoint, voltage));
        let next_current = midpoint + (midpoint - current);
        if !next_voltage.is_finite() || !next_current.is_finite() {
            return Err(StepError::State);
        }
        Ok(RlcState {
            voltage: ElectricPotential::from_base(next_voltage),
            current: ElectricCurrent::from_base(next_current),
        })
    }
}
