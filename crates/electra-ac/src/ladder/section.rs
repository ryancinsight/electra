//! Loaded section shared by signal transfer and independent source propagation.
use super::{Stage, divider};
use crate::{
    AcError, Complex64, Dimensionless, ElectricConductance, ElectricalImpedance, Frequency,
};

pub(super) fn check_frequency(frequency: Frequency) -> Result<(), AcError> {
    if !frequency.as_base().is_finite() || frequency.into_base() < 0. {
        Err(AcError::Frequency)
    } else {
        Ok(())
    }
}

impl Stage {
    #[inline]
    pub(super) fn loaded(
        self,
        gain: Dimensionless<Complex64>,
        upstream: ElectricalImpedance<Complex64>,
        frequency: Frequency,
        stage: usize,
    ) -> Result<(Dimensionless<Complex64>, ElectricalImpedance<Complex64>), AcError> {
        let series =
            upstream + ElectricalImpedance::from_base(Complex64::from(self.series.into_base()));
        let susceptance: ElectricConductance =
            (frequency * self.capacitance) * core::f64::consts::TAU;
        let admittance = ElectricConductance::from_base(Complex64::new(
            self.load.into_base(),
            susceptance.into_base(),
        ));
        let d = (Dimensionless::from_base(Complex64::from(1.)) + series * admittance).into_base();
        if !d.re.is_finite()
            || !d.im.is_finite()
            || (frequency.into_base() > 0.
                && self.capacitance.into_base() > 0.
                && susceptance.into_base() == 0.)
        {
            return Err(AcError::DynamicRange { stage });
        }
        let (gain, impedance) = divider::apply(gain, series, d);
        if [gain.into_base(), impedance.into_base()]
            .into_iter()
            .any(|z| !z.re.is_finite() || !z.im.is_finite() || (z.re == 0. && z.im == 0.))
        {
            return Err(AcError::DynamicRange { stage });
        }
        Ok((gain, impedance))
    }
}
