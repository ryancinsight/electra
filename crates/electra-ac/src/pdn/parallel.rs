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

/// Neumaier summation retains small terms between cancelling reactive branches.
#[derive(Default)]
struct Sum {
    value: f64,
    correction: f64,
}

impl Sum {
    fn add(&mut self, value: f64) {
        let next = self.value + value;
        self.correction += if self.value.abs() >= value.abs() {
            (self.value - next) + value
        } else {
            (value - next) + self.value
        };
        self.value = next;
    }

    fn total(self) -> f64 {
        self.value + self.correction
    }
}

#[derive(Default)]
pub(super) struct Parallel {
    conductance: Sum,
    susceptance: Sum,
}

impl Parallel {
    pub(super) fn add(&mut self, z: Complex64) -> Result<(), BankError> {
        let y = reciprocal(z)?;
        self.conductance.add(y.re);
        self.susceptance.add(y.im);
        Ok(())
    }

    pub(super) fn impedance(self) -> Result<PortImpedance, BankError> {
        let y = Complex64::new(self.conductance.total(), self.susceptance.total());
        if y.re == 0.0 && y.im == 0.0 {
            return Ok(PortImpedance::Open);
        }
        Ok(PortImpedance::Finite(ElectricalImpedance::from_base(
            reciprocal(y)?,
        )))
    }
}
