//! Cascaded Thévenin equivalents retain interstage loading.
mod divider;
pub mod noise;
mod section;
mod stage;
use crate::{AcError, AcPoint, Complex64, Dimensionless, ElectricalImpedance, Frequency};
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
        section::check_frequency(frequency)?;
        let one = Dimensionless::from_base(Complex64::from(1.));
        let mut gain = one;
        let mut impedance = ElectricalImpedance::<Complex64>::default();
        for (stage, cell) in self.0.iter().enumerate() {
            (gain, impedance) = cell.loaded(gain, impedance, frequency, stage)?;
        }
        Ok(AcPoint {
            frequency,
            gain,
            output_impedance: impedance,
        })
    }
}
