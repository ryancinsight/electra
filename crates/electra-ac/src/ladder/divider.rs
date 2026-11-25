//! Apply the same normalized complex divisor to gain and impedance.
use crate::{Complex64, Dimensionless, ElectricalImpedance};

type Gain = Dimensionless<Complex64>;
type Impedance = ElectricalImpedance<Complex64>;

pub(super) fn apply(mut gain: Gain, mut series: Impedance, mut d: Complex64) -> (Gain, Impedance) {
    // A passive ladder gives Re(1 + Zs*Y) >= 1. The ordinary reciprocal
    // is safe up to this threshold: squared components and the reciprocal
    // squared norm remain normal. Larger divisors need scaled numerators
    // to preserve representable output-impedance quadratures.
    let largest = d.re.abs().max(d.im.abs());
    if largest > 1e150 {
        let inverse_scale = largest.recip();
        gain *= inverse_scale;
        series *= inverse_scale;
        d *= inverse_scale;
    }
    let inverse_norm = (d.re * d.re + d.im * d.im).recip();
    let inverse = Gain::from_base(Complex64::new(d.re * inverse_norm, -d.im * inverse_norm));
    (gain * inverse, series * inverse)
}
