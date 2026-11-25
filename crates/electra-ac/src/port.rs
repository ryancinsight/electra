use crate::{Complex64, ElectricalImpedance};

/// Impedance looking into the positive port terminal with respect to its return.
#[derive(Clone, Copy, Debug)]
pub enum PortImpedance {
    /// Zero port admittance.
    Open,
    /// Complex impedance in Ω, including zero for an ideal short circuit.
    Finite(ElectricalImpedance<Complex64>),
}

impl PortImpedance {
    /// Magnitude in Ω. An open port has positive infinite magnitude.
    pub fn magnitude(self) -> ElectricalImpedance {
        ElectricalImpedance::from_base(match self {
            Self::Open => f64::INFINITY,
            Self::Finite(z) => z.as_base().re.hypot(z.as_base().im),
        })
    }
}
