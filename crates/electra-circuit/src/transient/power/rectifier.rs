use super::*;
use crate::semiconductor::JunctionDiode;

/// A series diode feeding a parallel capacitor and load resistor.
pub struct HalfWaveRectifier {
    diode: JunctionDiode,
    c: f64,
    r: f64,
}
impl HalfWaveRectifier {
    /// Construct the stage with positive capacitance and resistance.
    pub fn new(
        diode: JunctionDiode,
        c: Capacitance,
        r: ElectricalImpedance,
    ) -> Result<Self, PowerError> {
        let (c, r) = (c.into_base(), r.into_base());
        if positive(c) && positive(r) {
            Ok(Self { diode, c, r })
        } else {
            Err(PowerError::Input)
        }
    }
    /// Backward Euler at the endpoint source voltage. Safeguarded Newton solves
    /// capacitor and load KCL within the requested absolute current tolerance.
    pub fn advance(
        &self,
        previous: ElectricPotential,
        source: ElectricPotential,
        dt: Time,
        tolerance: ElectricCurrent,
        iterations: usize,
    ) -> Result<ElectricPotential, PowerError> {
        let dt = interval(dt)?;
        let (old, vs, tol) = (
            previous.into_base(),
            source.into_base(),
            tolerance.into_base(),
        );
        if !old.is_finite() || !vs.is_finite() || !positive(tol) {
            return Err(PowerError::Input);
        }
        let g = self.c / dt;
        if !g.is_finite() {
            return Err(PowerError::TimestepExhausted);
        }
        let mut low = old.min(vs).min(0.);
        let mut high = old.max(vs).max(0.);
        let mut v = old;
        for _ in 0..iterations {
            if let Some(point) = self.diode.at(ElectricPotential::from_base(vs - v)) {
                let residual = g.mul_add(v - old, v / self.r) - point.current.into_base();
                if residual.abs() <= tol {
                    return Ok(ElectricPotential::from_base(v));
                }
                if residual.is_nan() {
                    return Err(PowerError::Range);
                }
                if residual > 0. {
                    high = v;
                } else {
                    low = v;
                }
                let correction =
                    newton_correction(residual, g + 1. / self.r + point.conductance.into_base())?;
                let next = v - correction;
                if next > low && next < high {
                    v = next;
                    continue;
                }
            } else if vs > v {
                low = v;
            } else {
                return Err(PowerError::Range);
            }
            v = 0.5 * low + 0.5 * high;
        }
        Err(PowerError::NewtonLimit)
    }
}

fn newton_correction(residual: f64, pivot: f64) -> Result<f64, PowerError> {
    if pivot == 0. {
        Err(PowerError::Singular)
    } else if !pivot.is_finite() {
        Err(PowerError::Range)
    } else {
        Ok(residual / pivot)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_jacobian_is_singular() {
        assert_eq!(newton_correction(1., 0.), Err(PowerError::Singular));
    }
}
