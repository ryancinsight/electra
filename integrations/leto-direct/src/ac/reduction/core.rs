//! Map the remaining sparse rows into Leto's bounded real pivoted LU.
mod band;
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
    let mut slots = vec![usize::MAX; rows.len()];
    let mut nodes = Vec::new();
    for start in 0..rows.len() {
        if rows[start].is_none() || slots[start] != usize::MAX {
            continue;
        }
        nodes.clear();
        nodes.push(start);
        slots[start] = 0;
        let mut cursor = 0;
        while cursor < nodes.len() {
            let node = nodes[cursor];
            for &next in rows[node].as_ref().unwrap().edges.keys() {
                if slots[next] == usize::MAX {
                    slots[next] = nodes.len();
                    nodes.push(next);
                }
            }
            cursor += 1;
        }
        if nodes.len() <= limit {
            solve_block(rows, free, volts, &slots, &nodes)?;
        } else {
            band::solve(rows, free, volts, &slots, &nodes)?;
        }
    }
    Ok(())
}

fn solve_block(
    rows: &[Option<Row>],
    free: &[usize],
    volts: &mut [ElectricPotential<Complex64>],
    slots: &[usize],
    nodes: &[usize],
) -> Result<(), Box<dyn Error>> {
    let count = nodes.len();
    let order = count * 2;
    let mut matrix = vec![0.; order * order];
    let mut rhs = vec![0.; order];
    for &node in nodes {
        let row = rows[node].as_ref().unwrap();
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
    for &node in nodes {
        let i = 2 * slots[node];
        if !x[i].is_finite() || !x[i + 1].is_finite() {
            return Err("nonfinite AC core solution".into());
        }
        volts[free[node]] = ElectricPotential::from_base(Complex64::new(x[i], x[i + 1]));
    }
    Ok(())
}
