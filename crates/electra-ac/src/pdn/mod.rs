//! Driving-point impedance of a parallel capacitor bank.
mod branch;
mod parallel;

use crate::{Complex64, ElectricalImpedance, Frequency};
pub use branch::SeriesCapacitor;

/// Impedance looking into the positive bank terminal with the return grounded.
#[derive(Clone, Copy, Debug)]
pub enum PortImpedance {
    /// Zero port admittance, including an empty bank or capacitors at DC.
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

/// Invalid physical input or unrepresentable arithmetic in a bank evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BankError {
    /// Capacitance must be finite and strictly positive.
    Capacitance,
    /// Equivalent series resistance must be finite and nonnegative.
    Resistance,
    /// Equivalent series inductance must be finite and nonnegative.
    Inductance,
    /// Frequency must be finite and nonnegative.
    Frequency,
    /// A branch reactance, admittance or result exceeds the scalar range.
    DynamicRange,
}

impl std::fmt::Display for BankError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Capacitance => "capacitance must be finite and positive",
            Self::Resistance => "ESR must be finite and nonnegative",
            Self::Inductance => "ESL must be finite and nonnegative",
            Self::Frequency => "frequency must be finite and nonnegative",
            Self::DynamicRange => "bank impedance calculation exceeded the numeric range",
        })
    }
}
impl std::error::Error for BankError {}

/// Parallel connection of borrowed, prevalidated series branches.
#[derive(Clone, Copy, Debug)]
pub struct CapacitorBank<'a>(&'a [SeriesCapacitor]);

impl<'a> CapacitorBank<'a> {
    /// Borrow the branches in their input order. An empty bank is an open port.
    pub fn new(branches: &'a [SeriesCapacitor]) -> Self {
        Self(branches)
    }

    /// Evaluate one frequency in O(branches) time and constant auxiliary storage.
    pub fn at(self, frequency: Frequency) -> Result<PortImpedance, BankError> {
        if !frequency.as_base().is_finite() || frequency.into_base() < 0.0 {
            return Err(BankError::Frequency);
        }
        if frequency.into_base() == 0.0 {
            return Ok(PortImpedance::Open);
        }
        if let [branch] = self.0 {
            return branch
                .impedance(frequency)
                .map(|z| PortImpedance::Finite(ElectricalImpedance::from_base(z)));
        }
        let mut parallel = parallel::Parallel::default();
        let mut unresolved = false;
        for &branch in self.0 {
            match branch.impedance(frequency) {
                Ok(z) if z.re == 0.0 && z.im == 0.0 => {
                    return Ok(PortImpedance::Finite(ElectricalImpedance::default()));
                }
                Ok(z) => unresolved |= parallel.add(z).is_err(),
                Err(_) => unresolved = true,
            }
        }
        if unresolved {
            return Err(BankError::DynamicRange);
        }
        parallel.impedance()
    }
}
