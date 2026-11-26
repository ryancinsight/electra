//! Row equilibration and real representation of complex admittance rows.
use electra_circuit::AcSystem;
use std::error::Error;

pub(super) struct Block {
    pub matrix: Vec<f64>,
    pub rhs: Vec<f64>,
}

fn scaled(v: f64, scale: f64) -> Result<f64, Box<dyn Error>> {
    let value = v / scale;
    if !value.is_finite() || (value == 0. && v != 0.) {
        return Err("AC row scaling exceeds numeric range".into());
    }
    Ok(value)
}

impl Block {
    pub fn new(system: &AcSystem, free: &[usize], slots: &[usize]) -> Result<Self, Box<dyn Error>> {
        let order = 2 * free.len();
        let mut matrix = vec![0.; order * order];
        let mut rhs = vec![0.; order];
        for (r, &i) in free.iter().enumerate() {
            let scale = system.row(i).fold(0.0_f64, |scale, (_, y)| {
                let z = y.into_base();
                scale.max(z.re.abs()).max(z.im.abs())
            });
            if scale == 0. {
                return Err("singular AC equation with zero admittance".into());
            }
            for (j, y) in system.row(i) {
                let c = slots[j];
                let z = y.into_base();
                let a = scaled(z.re, scale)?;
                let b = scaled(z.im, scale)?;
                let k = 2 * r * order + 2 * c;
                matrix[k] = a;
                matrix[k + 1] = -b;
                matrix[k + order] = b;
                matrix[k + order + 1] = a;
            }
            let z = system
                .currents()
                .nth(i)
                .expect("valid original node")
                .into_base();
            rhs[2 * r] = scaled(z.re, scale)?;
            rhs[2 * r + 1] = scaled(z.im, scale)?;
        }
        Ok(Self { matrix, rhs })
    }
}
