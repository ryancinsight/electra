//! Leaf-to-root Schur elimination on a breadth-first ordered admittance tree.
use super::scaling::{row_scale, scaled};
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

fn quotient(a: Complex64, b: Complex64) -> Result<Complex64, Box<dyn Error>> {
    let scale = b.re.abs().max(b.im.abs());
    if scale == 0. || !scale.is_finite() {
        return Err("unusable AC tree pivot; a pivoted sparse solve is required".into());
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

pub(super) fn solve(
    system: &AcSystem,
    free: &[usize],
    slots: &[usize],
    volts: &mut [ElectricPotential<Complex64>],
) -> Result<(), Box<dyn Error>> {
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
                        return Err("cyclic AC component exceeds dense order limit".into());
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
    Ok(())
}
