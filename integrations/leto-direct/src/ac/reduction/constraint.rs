//! A zero-diagonal leaf directly prescribes its neighbor's voltage. Recover
//! the leaf later from that neighbor's original equation (a two-node pivot).
use super::{Recovery, Row, Step, quotient};
use electra_ac::{Complex64, ElectricPotential};
use std::{collections::VecDeque, error::Error};

pub(super) fn eliminate(
    i: usize,
    rows: &mut [Option<Row>],
    queue: &mut VecDeque<usize>,
    free: &[usize],
    volts: &mut [ElectricPotential<Complex64>],
) -> Result<Recovery, Box<dyn Error>> {
    let leaf = rows[i].take().unwrap();
    let (&j, to_parent) = leaf.edges.first_key_value().unwrap();
    let known = quotient(leaf.rhs.total(), to_parent.total())?;
    volts[free[j]] = ElectricPotential::from_base(known);
    let mut parent = rows[j].take().unwrap();
    let diagonal = parent.edges.remove(&i).unwrap().total();
    parent.rhs.add(-parent.diagonal.total() * known);
    let mut edges = Vec::with_capacity(parent.edges.len());
    for (k, y) in parent.edges {
        let neighbor = rows[k].as_mut().unwrap();
        let incoming = neighbor.edges.remove(&j).unwrap().total();
        neighbor.rhs.add(-incoming * known);
        edges.push((k, y.total()));
        if neighbor.edges.len() <= 2 {
            queue.push_back(k);
        }
    }
    Ok(Recovery::Constraint(Step {
        slot: i,
        diagonal,
        rhs: parent.rhs.total(),
        edges,
    }))
}
