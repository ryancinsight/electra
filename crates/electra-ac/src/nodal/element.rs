use super::StampError;
use crate::{Capacitance, Complex64, ElectricalImpedance, Frequency, Inductance};
use core::f64::consts::TAU;

/// Frequency-independent passive device. Resistor conductance is cached at construction.
#[derive(Clone, Copy, Debug)]
pub struct PassiveElement(Model);

#[derive(Clone, Copy, Debug)]
enum Model {
    Resistor(crate::ElectricConductance),
    Capacitor(Capacitance),
    Inductor(Inductance),
}

impl PassiveElement {
    /// Prepare a finite positive resistance in ohms.
    pub fn resistor(r: ElectricalImpedance) -> Result<Self, StampError> {
        let r = r.into_base();
        if !r.is_finite() || r <= 0. {
            return Err(StampError::Resistance);
        }
        let g = r.recip();
        if !g.is_finite() {
            return Err(StampError::Range);
        }
        Ok(Self(Model::Resistor(
            crate::ElectricConductance::from_base(g),
        )))
    }

    /// Prepare a finite nonnegative capacitance in farads. Zero represents an open.
    pub fn capacitor(c: Capacitance) -> Result<Self, StampError> {
        if !c.as_base().is_finite() || c.into_base() < 0. {
            return Err(StampError::Capacitance);
        }
        Ok(Self(Model::Capacitor(c)))
    }

    /// Prepare a finite positive inductance in henries.
    pub fn inductor(l: Inductance) -> Result<Self, StampError> {
        if !l.as_base().is_finite() || l.into_base() <= 0. {
            return Err(StampError::Inductance);
        }
        Ok(Self(Model::Inductor(l)))
    }

    pub(super) fn at(self, f: Frequency) -> Result<Complex64, StampError> {
        let hz = f.into_base();
        let b = match self.0 {
            Model::Resistor(g) => return Ok(Complex64::from(g.into_base())),
            Model::Capacitor(c) => {
                let c = c.into_base();
                if hz == 0. || c == 0. {
                    return Ok(Complex64::default());
                }
                // Scale the larger factor first to retain tiny positive products.
                let product = hz * c;
                if product < f64::MIN_POSITIVE {
                    (TAU * hz.max(c)) * hz.min(c)
                } else {
                    TAU * product
                }
            }
            Model::Inductor(l) => {
                if hz == 0. {
                    return Err(StampError::DcInductor);
                }
                let l = l.into_base();
                let product = hz * l;
                if product.is_infinite() {
                    -(TAU.recip() / hz.max(l)) / hz.min(l)
                } else {
                    -TAU.recip() / product
                }
            }
        };
        if !b.is_finite() || b == 0. {
            return Err(StampError::Range);
        }
        Ok(Complex64::new(0., b))
    }
}
