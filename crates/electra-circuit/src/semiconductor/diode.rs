use super::ThermodynamicTemperature;
use crate::{Dimensionless, ElectricConductance, ElectricCurrent, ElectricPotential};
use aequitas::systems::si::constants::{BOLTZMANN_CONSTANT, ELEMENTARY_CHARGE};

/// Ideal Shockley junction at a fixed temperature. Supply saturation current
/// at that temperature. Series resistance, breakdown and dynamic charge are excluded.
pub struct JunctionDiode {
    saturation: f64,
    slope_voltage: f64,
}

/// Current from anode to cathode and local derivative dI/dV.
pub struct DiodePoint {
    /// Branch current at the supplied junction voltage.
    pub current: ElectricCurrent,
    /// Differential conductance for a small-signal or Newton stamp.
    pub conductance: ElectricConductance,
}

impl JunctionDiode {
    /// Construct from positive finite saturation current, emission coefficient and kelvin.
    pub fn new(
        saturation: ElectricCurrent,
        emission: Dimensionless,
        temperature: ThermodynamicTemperature,
    ) -> Option<Self> {
        let saturation = saturation.into_base();
        let n = emission.into_base();
        let kelvin = temperature.into_base();
        if ![saturation, n, kelvin]
            .iter()
            .all(|v| v.is_finite() && *v > 0.)
        {
            return None;
        }
        let thermal_per_kelvin = BOLTZMANN_CONSTANT.into_base() / ELEMENTARY_CHARGE.into_base();
        let slope_voltage = (kelvin * thermal_per_kelvin) * n;
        (slope_voltage > 0. && slope_voltage.is_finite()).then_some(Self {
            saturation,
            slope_voltage,
        })
    }
    /// Evaluate current and conductance without allocating. Returns None for
    /// nonfinite voltage or a result outside the finite f64 range.
    pub fn at(&self, voltage: ElectricPotential) -> Option<DiodePoint> {
        let v = voltage.into_base();
        if !v.is_finite() {
            return None;
        }
        let x = v / self.slope_voltage;
        let (exponential, delta) = if x.abs() < 0.5 {
            let delta = x.exp_m1();
            (1. + delta, delta)
        } else {
            let exponential = x.exp();
            (exponential, exponential - 1.)
        };
        let current = if exponential.is_finite() {
            self.saturation * delta
        } else {
            (self.saturation.ln() + x).exp() - self.saturation
        };
        let scaled = self.saturation / self.slope_voltage;
        let direct = scaled * exponential;
        let conductance = if direct.is_normal() && exponential.is_normal() && scaled.is_normal() {
            direct
        } else {
            ((self.saturation.ln() - self.slope_voltage.ln()) + x).exp()
        };
        (current.is_finite() && conductance.is_finite()).then_some(DiodePoint {
            current: ElectricCurrent::from_base(current),
            conductance: ElectricConductance::from_base(conductance),
        })
    }
}
