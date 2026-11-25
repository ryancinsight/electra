//! Two-terminal capacitor with constant series resistance and inductance.
use super::BankError;
use crate::{
    Capacitance, Complex64, ElectricConductance, ElectricalImpedance, Frequency, Inductance,
};

/// A passive series RLC branch with capacitance in F, ESR in Ω and ESL in H.
#[derive(Clone, Copy, Debug)]
pub struct SeriesCapacitor {
    capacitance: Capacitance,
    esr: ElectricalImpedance,
    esl: Inductance,
}

impl SeriesCapacitor {
    /// Construct a branch with positive capacitance and nonnegative ESR and ESL.
    ///
    /// ```compile_fail
    /// use electra_ac::{pdn::SeriesCapacitor, Capacitance, ElectricalImpedance, Inductance};
    /// let c = Capacitance::from_base(100e-9);
    /// let r = ElectricalImpedance::from_base(0.05);
    /// let l = Inductance::from_base(0.5e-9);
    /// let _ = SeriesCapacitor::new(l, r, c);
    /// ```
    pub fn new(
        capacitance: Capacitance,
        esr: ElectricalImpedance,
        esl: Inductance,
    ) -> Result<Self, BankError> {
        if !capacitance.as_base().is_finite() || capacitance.into_base() <= 0.0 {
            return Err(BankError::Capacitance);
        }
        if !esr.as_base().is_finite() || esr.into_base() < 0.0 {
            return Err(BankError::Resistance);
        }
        if !esl.as_base().is_finite() || esl.into_base() < 0.0 {
            return Err(BankError::Inductance);
        }
        Ok(Self {
            capacitance,
            esr,
            esl,
        })
    }

    /// Rectangular impedance for a strictly positive frequency under exp(+jωt).
    pub(super) fn impedance(self, frequency: Frequency) -> Result<Complex64, BankError> {
        let inductive: ElectricalImpedance = (frequency * self.esl) * std::f64::consts::TAU;
        let susceptance: ElectricConductance =
            (frequency * self.capacitance) * std::f64::consts::TAU;
        let capacitive = susceptance.into_base().recip();
        let reactance = inductive.into_base() - capacitive;
        if !reactance.is_finite()
            || capacitive == 0.0
            || (self.esl.into_base() != 0.0 && inductive.into_base() == 0.0)
        {
            return Err(BankError::DynamicRange);
        }
        Ok(Complex64::new(self.esr.into_base(), reactance))
    }
}
