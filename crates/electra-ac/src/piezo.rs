//! Nominal Butterworth–Van Dyke electrical port with one motional mode.
use crate::{
    Capacitance, ElectricalImpedance, Frequency, Inductance, PortImpedance,
    pdn::{BankError, CapacitorBank, SeriesCapacitor},
};

/// Motional RLC in parallel with the clamped capacitance.
#[derive(Clone, Copy, Debug)]
pub struct BvdLoad([SeriesCapacitor; 2]);
impl BvdLoad {
    /// Construct a single-mode load. Motional inductance and both capacitances must be positive.
    pub fn new(
        r: ElectricalImpedance,
        l: Inductance,
        c: Capacitance,
        clamped: Capacitance,
    ) -> Result<Self, BankError> {
        if l.into_base() <= 0. {
            return Err(BankError::Inductance);
        }
        Ok(Self([
            SeriesCapacitor::new(c, r, l)?,
            SeriesCapacitor::new(
                clamped,
                ElectricalImpedance::default(),
                Inductance::default(),
            )?,
        ]))
    }
    /// Electrical input impedance at the electrodes, including the open DC limit.
    pub fn at(&self, frequency: Frequency) -> Result<PortImpedance, BankError> {
        CapacitorBank::new(&self.0).at(frequency)
    }
}
