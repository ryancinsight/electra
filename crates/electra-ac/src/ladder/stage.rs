use crate::{AcError, Capacitance, ElectricConductance, ElectricalImpedance};

/// Series resistor followed by a parallel capacitor and conductance to ground.
///
/// ```compile_fail
/// use electra_ac::{Capacitance, ElectricPotential, ElectricConductance, Stage};
/// let _ = Stage::new(ElectricPotential::from_base(1.), Capacitance::from_base(1e-6),
///                    ElectricConductance::from_base(0.));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Stage {
    pub(super) series: ElectricalImpedance,
    pub(super) capacitance: Capacitance,
    pub(super) load: ElectricConductance,
}
impl Stage {
    /// Construct a passive stage. Either shunt element may be zero.
    pub fn new(
        series: ElectricalImpedance,
        capacitance: Capacitance,
        load: ElectricConductance,
    ) -> Result<Self, AcError> {
        if !series.as_base().is_finite() || series.into_base() <= 0. {
            return Err(AcError::Resistance);
        }
        if !capacitance.as_base().is_finite() || capacitance.into_base() < 0. {
            return Err(AcError::Capacitance);
        }
        if !load.as_base().is_finite() || load.into_base() < 0. {
            return Err(AcError::Conductance);
        }
        Ok(Self {
            series,
            capacitance,
            load,
        })
    }
}
