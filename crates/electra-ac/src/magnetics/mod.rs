//! Voltage-driven linear transformers with a resistive or open secondary.
mod coupled;
mod ideal;

use crate::{Complex64, Dimensionless, ElectricalImpedance, Frequency, PortImpedance};
pub use coupled::CoupledInductors;
pub use ideal::IdealTransformer;

/// Load connected across the secondary terminals.
#[derive(Clone, Copy, Debug)]
pub enum SecondaryLoad {
    /// No secondary current.
    Open,
    /// Finite nonnegative load in Ω. Zero denotes a short circuit.
    Resistance(ElectricalImpedance),
}

impl SecondaryLoad {
    fn validate(self) -> Result<Self, MagneticError> {
        if let Self::Resistance(r) = self
            && (!r.as_base().is_finite() || r.into_base() < 0.)
        {
            return Err(MagneticError::Load);
        }
        Ok(self)
    }
}

/// Response to a nonzero ideal primary voltage phasor, with exp(+jωt).
#[derive(Clone, Copy, Debug)]
pub struct TransformerPoint {
    /// Loaded secondary voltage divided by primary voltage, with the declared dot polarity.
    pub gain: Dimensionless<Complex64>,
    /// Impedance looking into the driven primary terminals.
    pub input_impedance: PortImpedance,
}

/// Invalid winding, termination, frequency or numerical range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MagneticError {
    /// Both self-inductances must be finite, positive henries.
    Inductance,
    /// Signed coupling must be finite and lie in [−1, 1].
    Coupling,
    /// The signed secondary-to-primary ideal turns ratio must be finite and nonzero.
    TurnsRatio,
    /// A resistive termination must be finite and nonnegative.
    Load,
    /// AC phasor frequency must be finite and strictly positive.
    Frequency,
    /// A perfectly coupled, shorted secondary is incompatible with a nonzero primary voltage.
    ShortedIdeal,
    /// A required coefficient, reactance, or result exceeds the numeric range.
    DynamicRange,
}

impl std::fmt::Display for MagneticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Inductance => "self-inductance must be finite and positive in henries",
            Self::Coupling => "signed coupling must lie in [-1, 1]",
            Self::TurnsRatio => "signed turns ratio must be finite and nonzero",
            Self::Load => "secondary resistance must be finite and nonnegative",
            Self::Frequency => "transformer phasor frequency must be finite and positive",
            Self::ShortedIdeal => {
                "an ideal secondary short is incompatible with a nonzero primary voltage"
            }
            Self::DynamicRange => "transformer calculation exceeded the numeric range",
        })
    }
}
impl std::error::Error for MagneticError {}

fn check_frequency(frequency: Frequency) -> Result<(), MagneticError> {
    if frequency.as_base().is_finite() && frequency.into_base() > 0. {
        Ok(())
    } else {
        Err(MagneticError::Frequency)
    }
}
