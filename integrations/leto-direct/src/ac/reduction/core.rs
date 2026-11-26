//! Map the remaining sparse rows into Leto's bounded real pivoted LU.
use super::Row;
use crate::{PivotedLu, ac::block};
use electra_ac::{Complex64, ElectricPotential};
use std::error::Error;

pub(super) fn solve(
    rows: &[Option<Row>],
    free: &[usize],
    volts: &mut [ElectricPotential<Complex64>],
    limit: usize,
) -> Result<(), Box<dyn Error>> {
    let count = rows.iter().flatten().count();
    if count > limit {
        return Err(
            format!("irreducible AC core of {count} nodes exceeds pivoted limit {limit}").into(),
        );
    }
    let mut slots = vec![0; rows.len()];
    for (i, (node, _)) in rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.is_some())
        .enumerate()
    {
        slots[node] = i;
    }
    let order = count * 2;
    let mut matrix = vec![0.; order * order];
    let mut rhs = vec![0.; order];
    for (node, row) in rows.iter().enumerate() {
        let Some(row) = row else { continue };
        let i = slots[node];
        let d = row.diagonal.total();
        block::put(&mut matrix, order, i, i, d.re, d.im);
        for (&j, entry) in &row.edges {
            let z = entry.total();
            block::put(&mut matrix, order, i, slots[j], z.re, z.im);
        }
        let b = row.rhs.total();
        rhs[2 * i] = b.re;
        rhs[2 * i + 1] = b.im;
    }
    let mut x = vec![0.; order];
    PivotedLu::from_values(order, matrix)?.solve(&rhs, &mut x)?;
    for (node, row) in rows.iter().enumerate() {
        if row.is_some() {
            let i = 2 * slots[node];
            if !x[i].is_finite() || !x[i + 1].is_finite() {
                return Err("nonfinite AC core solution".into());
            }
            volts[free[node]] = ElectricPotential::from_base(Complex64::new(x[i], x[i + 1]));
        }
    }
    Ok(())
}
