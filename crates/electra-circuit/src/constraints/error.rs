use crate::ElectricPotential;
use std::fmt;

/// The source card index and the electrical reason that unique currents cannot be reported.
pub(crate) struct VoltageError {
    pub source: usize,
    pub reason: VoltageFault,
}
pub(crate) enum VoltageFault {
    Redundant,
    Parallel {
        requested: ElectricPotential,
        imposed: ElectricPotential,
    },
    Incompatible(ElectricPotential),
    Range,
}
impl fmt::Display for VoltageFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Redundant => f.write_str(
                "redundant ideal voltage loop: individual source currents are not unique",
            ),
            Self::Parallel { requested, imposed } => write!(
                f,
                "incompatible ideal voltage loop: requested {:e} V, parallel source imposes {:e} V",
                requested.as_base(),
                imposed.as_base()
            ),
            Self::Incompatible(error) => write!(
                f,
                "incompatible ideal voltage loop: closure error {:e} V",
                error.as_base()
            ),
            Self::Range => f.write_str("voltage loop closure exceeds floating-point range"),
        }
    }
}
