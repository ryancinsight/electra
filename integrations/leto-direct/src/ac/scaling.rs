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
