//! Leaf-to-root Schur elimination on a breadth-first ordered admittance tree.
use super::scaling::{quotient, row_scale, scaled};
use electra_ac::{Complex64, ElectricPotential, PhasorSum};
use electra_circuit::AcSystem;
use std::error::Error;

#[derive(Default, Clone, Copy)]
struct Row {
    parent: Option<usize>,
    to_parent: Complex64,
    from_parent: Complex64,
    diagonal: PhasorSum,
    rhs: PhasorSum,
}

pub(super) fn try_solve(
    system: &AcSystem,
    free: &[usize],
    slots: &[usize],
    volts: &mut [ElectricPotential<Complex64>],
) -> Result<bool, Box<dyn Error>> {
    let mut rows = vec![Row::default(); free.len()];
    for (r, &i) in free.iter().enumerate() {
        let scale = row_scale(system, i)?;
        let normalize = |z: Complex64| -> Result<Complex64, Box<dyn Error>> {
            Ok(Complex64::new(scaled(z.re, scale)?, scaled(z.im, scale)?))
        };
        rows[r].rhs.add(normalize(
            system.currents().nth(i).expect("valid node").into_base(),
        )?);
        for (j, y) in system.row(i) {
            let c = slots[j];
            let value = normalize(y.into_base())?;
            match c.cmp(&r) {
                std::cmp::Ordering::Equal => rows[r].diagonal.add(value),
                std::cmp::Ordering::Less => {
                    if rows[r].parent.replace(c).is_some() {
                        return Ok(false);
                    }
                    rows[r].to_parent = value;
                }
                std::cmp::Ordering::Greater => rows[c].from_parent = value,
            }
        }
    }
    for r in (1..rows.len()).rev() {
        let row = rows[r];
        let p = row
            .parent
            .expect("each non-root traversal node has an earlier neighbor");
        if row.diagonal.total() == Complex64::default() {
            return Ok(false);
        }
        let multiplier = quotient(row.from_parent, row.diagonal.total())?;
        rows[p].diagonal.add(-multiplier * row.to_parent);
        rows[p].rhs.add(-multiplier * row.rhs.total());
    }
    for (r, row) in rows.iter().enumerate() {
        let mut rhs = row.rhs;
        if let Some(p) = row.parent {
            rhs.add(-row.to_parent * volts[free[p]].into_base());
        }
        volts[free[r]] = ElectricPotential::from_base(quotient(rhs.total(), row.diagonal.total())?);
    }
    Ok(true)
}
