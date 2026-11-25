//! Prepare and evaluate a linear pair of lossless, magnetically coupled windings.
mod response;

use super::{MagneticError, SecondaryLoad, TransformerPoint, check_frequency};
use crate::{Complex64, Dimensionless, ElectricalImpedance, Frequency, Inductance, PortImpedance};

/// Linear windings with finite self-inductances, signed coupling and a fixed secondary load.
/// Positive coupling gives equal open-circuit voltage polarity at the indicated terminals.
#[derive(Clone, Copy, Debug)]
pub struct CoupledInductors {
    primary: Inductance,
    primary_fraction: Dimensionless,
    gain: Dimensionless,
    load: Option<ResistiveSecondary>,
}

/// Retained only when the load feeds back through mutual inductance.
#[derive(Clone, Copy, Debug)]
struct ResistiveSecondary {
    inductance: Inductance,
    resistance: ElectricalImpedance,
    coupling: Dimensionless,
}

impl CoupledInductors {
    /// Prepare the winding ratio and leakage fraction once for a sweep.
    ///
    /// ```
    /// use electra_ac::{Dimensionless, ElectricalImpedance, Frequency, Inductance,
    ///     magnetics::{CoupledInductors, SecondaryLoad}};
    /// let windings = CoupledInductors::new(Inductance::from_base(0.001),
    ///     Inductance::from_base(0.004), Dimensionless::from_base(-1.),
    ///     SecondaryLoad::Resistance(ElectricalImpedance::from_base(40.)))?;
    /// assert_eq!(windings.at(Frequency::from_base(1000.))?.gain.as_base().re, -2.);
    /// # Ok::<(), electra_ac::magnetics::MagneticError>(())
    /// ```
    ///
    /// ```compile_fail
    /// use electra_ac::{Dimensionless, ElectricalImpedance, Inductance,
    ///     magnetics::{CoupledInductors, SecondaryLoad}};
    /// let _ = CoupledInductors::new(ElectricalImpedance::from_base(40.),
    ///     Inductance::from_base(0.004), Dimensionless::from_base(1.), SecondaryLoad::Open);
    /// ```
    pub fn new(
        primary: Inductance,
        secondary: Inductance,
        coupling: Dimensionless,
        load: SecondaryLoad,
    ) -> Result<Self, MagneticError> {
        if [primary, secondary]
            .into_iter()
            .any(|l| !l.as_base().is_finite() || l.into_base() <= 0.)
        {
            return Err(MagneticError::Inductance);
        }
        let k = coupling.into_base();
        if !k.is_finite() || k.abs() > 1. {
            return Err(MagneticError::Coupling);
        }
        let load = load.validate()?;
        let leakage = Dimensionless::from_base((1. - k.abs()) * (1. + k.abs()));
        let one = Dimensionless::from_base(1.);
        let (gain, primary_fraction, load) = match load {
            SecondaryLoad::Resistance(r) if r.into_base() == 0. => {
                if k.abs() == 1. {
                    return Err(MagneticError::ShortedIdeal);
                }
                // A short leaves only primary leakage inductance. Folding it
                // now also permits finite jωLleak when jωLp would overflow.
                let effective: Inductance = primary * leakage;
                if effective.into_base() >= f64::MIN_POSITIVE {
                    return Ok(Self {
                        primary: effective,
                        primary_fraction: one,
                        gain: Dimensionless::default(),
                        load: None,
                    });
                }
                // Keep subnormal leakage factored until frequency is known.
                (Dimensionless::default(), leakage, None)
            }
            _ if k == 0. => (Dimensionless::default(), one, None),
            SecondaryLoad::Open => (
                response::open_gain(primary, secondary, coupling)?,
                one,
                None,
            ),
            SecondaryLoad::Resistance(resistance) => (
                response::open_gain(primary, secondary, coupling)?,
                leakage,
                Some(ResistiveSecondary {
                    inductance: secondary,
                    coupling,
                    resistance,
                }),
            ),
        };
        Ok(Self {
            primary,
            primary_fraction,
            gain,
            load,
        })
    }

    /// Compute complex voltage gain and primary input impedance with no sweep workspace.
    pub fn at(&self, frequency: Frequency) -> Result<TransformerPoint, MagneticError> {
        check_frequency(frequency)?;
        let xp = response::reactance(frequency, self.primary)?;
        let direct: ElectricalImpedance = xp * self.primary_fraction;
        let (gain, zin) = if let Some(load) = self.load {
            let xs = response::reactance(frequency, load.inductance)?;
            let leakage: ElectricalImpedance = xs * self.primary_fraction;
            if self.primary_fraction.into_base() > 0. && leakage.into_base() == 0. {
                return Err(MagneticError::DynamicRange);
            }
            let h = response::loaded(self.gain, load.resistance, leakage).into_base();
            if h.re == 0. && h.im == 0. {
                return Err(MagneticError::DynamicRange);
            }
            // Zin = j Xp (1-k²) + j Xp k² R/(R+jXs). Both contributions are passive.
            let shared: ElectricalImpedance = (xp * load.coupling) * load.coupling;
            let reflected = response::loaded(shared, load.resistance, xs).into_base();
            (
                h,
                Complex64::new(-reflected.im, direct.into_base() + reflected.re),
            )
        } else {
            (
                Complex64::from(self.gain.into_base()),
                Complex64::new(0., direct.into_base()),
            )
        };
        if !zin.im.is_finite() || (zin.re == 0. && zin.im == 0.) {
            return Err(MagneticError::DynamicRange);
        }
        Ok(TransformerPoint {
            gain: Dimensionless::from_base(gain),
            input_impedance: PortImpedance::Finite(ElectricalImpedance::from_base(zin)),
        })
    }
}
