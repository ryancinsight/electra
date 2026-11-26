//! Row equilibration and real representation of complex admittance rows.
use electra_circuit::AcSystem;
use std::error::Error;

pub(super) struct Block {
    pub matrix: Vec<f64>,
    pub rhs: Vec<f64>,
}

use super::scaling::{row_scale, scaled};
impl Block {
    pub fn new(system: &AcSystem, free: &[usize], slots: &[usize]) -> Result<Self, Box<dyn Error>> {
        let order = 2 * free.len();
        let mut matrix = vec![0.; order * order];
        let mut rhs = vec![0.; order];
        for (r, &i) in free.iter().enumerate() {
            let scale = row_scale(system, i)?;
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
