//! Diagnose the independent-source loops excluded from a unique branch-current solution.
use super::{
    error::{VoltageError, VoltageFault},
    forest::TreeEdge,
};
use crate::{ElectricPotential, Element, Part};

pub(super) fn diagnose(
    n: usize,
    parts: &[Part],
    tree: &[TreeEdge],
    chords: &[usize],
) -> VoltageError {
    let mut parent = vec![None; n];
    let mut depth = vec![0; parent.len()];
    for edge in tree {
        parent[edge.child] = Some(edge);
        depth[edge.child] = depth[edge.parent] + 1;
    }
    for &index in chords {
        let part = &parts[index];
        let Element::Voltage { volts } = part.element else {
            unreachable!()
        };
        let (mut a, mut b) = (part.positive, part.negative);
        // Parallel sources assert the very same difference. Equality needs no summation tolerance.
        let direct = parent[a]
            .filter(|e| e.parent == b)
            .or_else(|| parent[b].filter(|e| e.parent == a));
        if let Some(edge) = direct {
            let p = &parts[edge.source];
            let Element::Voltage {
                volts: reference, ..
            } = p.element
            else {
                unreachable!()
            };
            let implied = p.step_to(a, reference);
            if implied != volts {
                return VoltageError {
                    source: index,
                    reason: VoltageFault::Parallel {
                        requested: volts,
                        imposed: implied,
                    },
                };
            }
            continue;
        }
        let mut sum = -volts;
        let mut correction = ElectricPotential::default();
        let mut smallest = if volts.into_base() == 0.0 {
            ElectricPotential::from_base(f64::INFINITY)
        } else {
            volts.abs()
        };
        while a != b {
            let side_a = depth[a] >= depth[b];
            let node = if side_a { &mut a } else { &mut b };
            let edge = parent[*node].expect("cycle endpoints have the same tree root");
            let p = &parts[edge.source];
            let Element::Voltage { volts, .. } = p.element else {
                unreachable!()
            };
            let step = p.step_to(*node, volts);
            let term = if side_a { step } else { -step };
            let next = sum + term;
            correction += if sum.abs() >= term.abs() {
                (sum - next) + term
            } else {
                (term - next) + sum
            };
            if term.into_base() != 0.0 {
                smallest = smallest.min(term.abs());
            }
            sum = next;
            *node = edge.parent;
        }
        let residual = sum + correction;
        if !residual.as_base().is_finite() {
            return VoltageError {
                source: index,
                reason: VoltageFault::Range,
            };
        }
        // The smallest nonzero voltage on this loop keeps the ambiguity band local,
        // including when a small source is connected to a large common-mode supply.
        if residual.abs() > smallest * (8.0 * f64::EPSILON) {
            return VoltageError {
                source: index,
                reason: VoltageFault::Incompatible(residual),
            };
        }
    }
    VoltageError {
        source: chords[0],
        reason: VoltageFault::Redundant,
    }
}
