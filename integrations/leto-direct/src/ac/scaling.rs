use electra_circuit::AcSystem;
use std::error::Error;
pub(super) fn scaled(v: f64, scale: f64) -> Result<f64, Box<dyn Error>> {
    let value = v / scale;
    if !value.is_finite() || (value == 0. && v != 0.) {
        return Err("AC row scaling exceeds numeric range".into());
    }
    Ok(value)
}

pub(super) fn row_scale(system: &AcSystem, i: usize) -> Result<f64, Box<dyn Error>> {
    let scale = system.row(i).fold(0.0_f64, |scale, (_, y)| {
        let z = y.into_base();
        scale.max(z.re.abs()).max(z.im.abs())
    });
    if scale == 0. {
        return Err("singular AC equation with zero admittance".into());
    }
    Ok(scale)
}

use electra_ac::Complex64;
pub(super) fn quotient(a: Complex64, b: Complex64) -> Result<Complex64, Box<dyn Error>> {
    let scale = b.re.abs().max(b.im.abs());
    if scale == 0. || !scale.is_finite() {
        return Err("unusable AC pivot".into());
    }
    let u = b.re / scale;
    let v = b.im / scale;
    let ar = scaled(a.re, scale)?;
    let ai = scaled(a.im, scale)?;
    let norm = u.mul_add(u, v * v);
    let z = Complex64::new(ar.mul_add(u, ai * v) / norm, ai.mul_add(u, -ar * v) / norm);
    if !z.re.is_finite() || !z.im.is_finite() {
        return Err("AC tree division exceeds numeric range".into());
    }
    Ok(z)
}
