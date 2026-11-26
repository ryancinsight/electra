use crate::Complex64;

/// Neumaier summation retains small terms between cancelling reactive branches.
#[derive(Default, Clone, Copy)]
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

/// Component-wise Neumaier accumulation for rectangular AC coefficients.
#[derive(Clone, Copy, Default)]
pub struct PhasorSum {
    real: Sum,
    imaginary: Sum,
}
impl PhasorSum {
    /// Add one complex contribution. The caller supplies consistent units.
    pub fn add(&mut self, z: Complex64) {
        self.real.add(z.re);
        self.imaginary.add(z.im);
    }
    /// Return the compensated complex total. Check finiteness before use.
    pub fn total(self) -> Complex64 {
        Complex64::new(self.real.total(), self.imaginary.total())
    }
}
