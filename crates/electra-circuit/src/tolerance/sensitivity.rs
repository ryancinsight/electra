//! Single-node adjoint of an unloaded resistive divider.
use crate::{ElectricCurrent, ElectricPotential, ElectricalImpedance};

/// Tap-voltage derivatives with respect to upper and lower resistance, in V/Ω.
pub struct DividerSensitivity {
    /// Change in tap voltage per ohm added to the upper resistor.
    pub upper: ElectricCurrent,
    /// Change in tap voltage per ohm added to the lower resistor.
    pub lower: ElectricCurrent,
}

impl DividerSensitivity {
    /// Solve the transposed one-node conductance equation for a unit tap probe.
    /// Each derivative is minus the product of forward and adjoint branch currents.
    pub fn at(
        source: ElectricPotential,
        upper: ElectricalImpedance,
        lower: ElectricalImpedance,
    ) -> Option<Self> {
        let (a, b) = (upper.into_base(), lower.into_base());
        if !source.into_base().is_finite() || !a.is_finite() || !b.is_finite() || a <= 0. || b <= 0.
        {
            return None;
        }
        let scale = a.max(b);
        let sum = a / scale + b / scale;
        let scaled = source.into_base() / scale;
        let current = if scaled.is_finite() {
            scaled / sum
        } else {
            (source.into_base() / sum) / scale
        };
        let upper = -current * ((b / scale) / sum);
        let lower = current * ((a / scale) / sum);
        (upper.is_finite() && lower.is_finite()).then_some(Self {
            upper: ElectricCurrent::from_base(upper),
            lower: ElectricCurrent::from_base(lower),
        })
    }
}
