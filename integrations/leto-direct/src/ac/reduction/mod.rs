//! Degree-limited Schur reduction with a bounded, pivoted dense remainder.
mod constraint;
mod core;
mod recovery;
use super::scaling::{quotient, row_scale, scaled};
use electra_ac::{Complex64, ElectricPotential, PhasorSum};
use electra_circuit::AcSystem;
use recovery::{Recovery, Step};
use std::{
    collections::{BTreeMap, VecDeque},
    error::Error,
};

#[derive(Default)]
struct Row {
    diagonal: PhasorSum,
    rhs: PhasorSum,
    edges: BTreeMap<usize, PhasorSum>,
}

pub(super) fn solve(
    system: &AcSystem,
    free: &[usize],
    slots: &[usize],
    volts: &mut [ElectricPotential<Complex64>],
    limit: usize,
) -> Result<(), Box<dyn Error>> {
    let mut rows = Vec::with_capacity(free.len());
    let mut queue = VecDeque::new();
    for (slot, &node) in free.iter().enumerate() {
        let mut row = Row::default();
        let scale = row_scale(system, node)?;
        let normalize = |z: Complex64| -> Result<Complex64, Box<dyn Error>> {
            Ok(Complex64::new(scaled(z.re, scale)?, scaled(z.im, scale)?))
        };
        row.rhs.add(normalize(
            system.currents().nth(node).expect("valid node").into_base(),
        )?);
        for (other, y) in system.row(node) {
            let value = normalize(y.into_base())?;
            if node == other {
                row.diagonal.add(value);
            } else {
                row.edges.entry(slots[other]).or_default().add(value);
            }
        }
        if row.edges.len() <= 2 {
            queue.push_back(slot);
        }
        rows.push(Some(row));
    }
    // Process exact one-neighbor voltage constraints before scalar pivots can
    // erase them through subtraction of nearly equal floating-point values.
    for (i, row) in rows.iter().enumerate() {
        let row = row.as_ref().unwrap();
        if row.edges.len() == 1 && row.diagonal.total() == Complex64::default() {
            queue.push_front(i);
        }
    }
    let mut remaining = rows.len();
    let mut steps = Vec::new();
    while let Some(i) = queue.pop_front() {
        if remaining <= 1 {
            break;
        }
        let Some(row) = &rows[i] else { continue };
        if row.edges.len() > 2 {
            continue;
        }
        let pivot = row.diagonal.total();
        if pivot == Complex64::default()
            && row.edges.len() == 1
            && row.edges.first_key_value().is_some_and(|(&j, y)| {
                y.total() != Complex64::default()
                    && rows[j].as_ref().unwrap().edges[&i].total() != Complex64::default()
            })
        {
            let step = constraint::eliminate(i, &mut rows, &mut queue, free, volts)?;
            steps.push(step);
            remaining -= 2;
            continue;
        }
        let magnitude = pivot.re.abs().max(pivot.im.abs());
        // Leave zero or weak column pivots for the pivoted core. Only a neighbor
        // update can change their eligibility, and that update requeues the row.
        if magnitude == 0.
            || row.edges.keys().any(|&j| {
                let incoming = rows[j].as_ref().unwrap().edges[&i].total();
                magnitude < 0.5 * incoming.re.abs().max(incoming.im.abs())
            })
        {
            continue;
        }
        let row = rows[i].take().unwrap();
        let mut edges = [None; 2];
        for (dst, (&j, y)) in edges.iter_mut().zip(&row.edges) {
            *dst = Some((j, y.total()));
        }
        let rhs = row.rhs.total();
        for &(j, _) in edges.iter().flatten() {
            let neighbor = rows[j].as_mut().unwrap();
            let incoming = neighbor.edges.remove(&i).unwrap().total();
            let multiplier = quotient(incoming, pivot)?;
            neighbor.rhs.add(-multiplier * rhs);
            for &(k, y) in edges.iter().flatten() {
                let target = if j == k {
                    &mut neighbor.diagonal
                } else {
                    neighbor.edges.entry(k).or_default()
                };
                target.add(-multiplier * y);
            }
            if neighbor.edges.len() <= 2 {
                queue.push_back(j);
            }
        }
        remaining -= 1;
        steps.push(Recovery::Scalar(Step {
            slot: i,
            diagonal: pivot,
            rhs,
            edges,
        }));
    }
    if remaining > 0 {
        core::solve(&rows, free, volts, limit)?;
    }
    for step in steps.into_iter().rev() {
        step.apply(free, volts)?;
    }
    Ok(())
}
