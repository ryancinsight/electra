//! Sum complex branch admittances, then invert once at the port.
use super::{BankError, PortImpedance};
use crate::{Complex64, ElectricalImpedance};

/// Scale before squaring so finite impedances around 1e±200 Ω are invertible.
fn reciprocal(z: Complex64) -> Result<Complex64, BankError> {
    let scale = z.re.abs().max(z.im.abs());
    if !scale.is_finite() || scale == 0.0 {
        return Err(BankError::DynamicRange);
    }
    let u = z.re / scale;
    let v = z.im / scale;
    let norm = u.mul_add(u, v * v);
    let inverse = Complex64::new((u / norm) / scale, (-v / norm) / scale);
    if !inverse.re.is_finite()
        || !inverse.im.is_finite()
        || (z.re != 0.0 && inverse.re == 0.0)
        || (z.im != 0.0 && inverse.im == 0.0)
    {
        return Err(BankError::DynamicRange);
    }
    Ok(inverse)
}

#[derive(Default)]
pub(super) struct Parallel {
    sum: crate::PhasorSum,
}

impl Parallel {
    pub(super) fn add(&mut self, z: Complex64) -> Result<(), BankError> {
        let y = reciprocal(z)?;
        self.sum.add(y);
        Ok(())
    }

    pub(super) fn impedance(self) -> Result<PortImpedance, BankError> {
        let y = self.sum.total();
        if y.re == 0.0 && y.im == 0.0 {
            return Ok(PortImpedance::Open);
        }
        Ok(PortImpedance::Finite(ElectricalImpedance::from_base(
            reciprocal(y)?,
        )))
    }
}
