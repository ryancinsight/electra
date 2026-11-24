//! Constitutive laws for linear dependent sources, in SI quantities.
use crate::{
    Dimensionless, ElectricConductance, ElectricCurrent, ElectricPotential, ElectricalImpedance,
    SourceValue,
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Controlled {
    VoltageGain([usize; 2], Dimensionless),
    Transconductance([usize; 2], ElectricConductance),
    CurrentGain(usize, Dimensionless),
    Transresistance(usize, ElectricalImpedance),
}

impl Controlled {
    pub fn voltage_output(self) -> bool {
        matches!(self, Self::VoltageGain(..) | Self::Transresistance(..))
    }

    pub fn response(
        self,
        potential: impl Fn(usize) -> ElectricPotential,
        current: impl Fn(usize) -> ElectricCurrent,
    ) -> SourceValue {
        match self {
            Self::VoltageGain([a, b], k) => SourceValue::Voltage((potential(a) - potential(b)) * k),
            Self::Transconductance([a, b], g) => {
                SourceValue::Current(g * (potential(a) - potential(b)))
            }
            Self::CurrentGain(s, k) => SourceValue::Current(current(s) * k),
            Self::Transresistance(s, r) => SourceValue::Voltage(r * current(s)),
        }
    }
}
