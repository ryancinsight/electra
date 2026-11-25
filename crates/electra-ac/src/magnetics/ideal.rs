//! Algebraic voltage ratio and load reflection for a lossless ideal transformer.
use super::{MagneticError, SecondaryLoad, TransformerPoint, check_frequency};
use crate::{Complex64, Dimensionless, ElectricalImpedance, Frequency, PortImpedance};

/// Ideal transformer with infinite magnetizing impedance and zero leakage.
#[derive(Clone, Copy, Debug)]
pub struct IdealTransformer(TransformerPoint);

impl IdealTransformer {
    /// Prepare the signed secondary-to-primary voltage ratio and input load reflection.
    pub fn new(ratio: Dimensionless, load: SecondaryLoad) -> Result<Self, MagneticError> {
        let n = ratio.into_base();
        if !n.is_finite() || n == 0. {
            return Err(MagneticError::TurnsRatio);
        }
        let input_impedance = match load.validate()? {
            SecondaryLoad::Open => PortImpedance::Open,
            SecondaryLoad::Resistance(r) => {
                if r.into_base() == 0. {
                    return Err(MagneticError::ShortedIdeal);
                }
                // The final reflection can be representable even when n² is not.
                let reflected: ElectricalImpedance = (r / ratio) / ratio;
                if !reflected.as_base().is_finite() || reflected.into_base() == 0. {
                    return Err(MagneticError::DynamicRange);
                }
                PortImpedance::Finite(ElectricalImpedance::from_base(Complex64::from(
                    reflected.into_base(),
                )))
            }
        };
        Ok(Self(TransformerPoint {
            gain: Dimensionless::from_base(Complex64::from(n)),
            input_impedance,
        }))
    }

    /// Evaluate the frequency-independent ideal response on the positive AC frequency axis.
    pub fn at(&self, frequency: Frequency) -> Result<TransformerPoint, MagneticError> {
        check_frequency(frequency)?;
        Ok(self.0)
    }
}
