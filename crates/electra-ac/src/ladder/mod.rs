//! Cascaded Thévenin equivalents retain interstage loading.
mod divider;
mod stage;
use crate::{
    AcError, AcPoint, Complex64, Dimensionless, ElectricConductance, ElectricalImpedance, Frequency,
};
pub use stage::Stage;

/// Ideal voltage source driving a borrowed, unbuffered ladder in input-to-output order.
#[derive(Clone, Copy, Debug)]
pub struct RcLadder<'a>(&'a [Stage]);

impl<'a> RcLadder<'a> {
    /// Borrow the validated stages without copying them.
    pub fn new(stages: &'a [Stage]) -> Result<Self, AcError> {
        if stages.is_empty() {
            Err(AcError::Empty)
        } else {
            Ok(Self(stages))
        }
    }

    /// Compute the final node's complex gain and Thévenin impedance at one frequency.
    /// The phasor convention is exp(+jωt). Zero hertz gives the DC limit.
    pub fn at(self, frequency: Frequency) -> Result<AcPoint, AcError> {
        if !frequency.as_base().is_finite() || frequency.into_base() < 0. {
            return Err(AcError::Frequency);
        }
        let one = Dimensionless::from_base(Complex64::from(1.));
        let mut gain = one;
        let mut impedance = ElectricalImpedance::<Complex64>::default();
        for (stage, cell) in self.0.iter().enumerate() {
            let series = impedance
                + ElectricalImpedance::from_base(Complex64::from(cell.series.into_base()));
            let susceptance: ElectricConductance =
                (frequency * cell.capacitance) * core::f64::consts::TAU;
            let admittance = ElectricConductance::from_base(Complex64::new(
                cell.load.into_base(),
                susceptance.into_base(),
            ));
            let d = (one + series * admittance).into_base();
            if !d.re.is_finite()
                || !d.im.is_finite()
                || (frequency.into_base() > 0.
                    && cell.capacitance.into_base() > 0.
                    && susceptance.into_base() == 0.)
            {
                return Err(AcError::DynamicRange { stage });
            }
            (gain, impedance) = divider::apply(gain, series, d);
            if [gain.into_base(), impedance.into_base()]
                .into_iter()
                .any(|z| !z.re.is_finite() || !z.im.is_finite() || (z.re == 0. && z.im == 0.))
            {
                return Err(AcError::DynamicRange { stage });
            }
        }
        Ok(AcPoint {
            frequency,
            gain,
            output_impedance: impedance,
        })
    }
}
