//! Grid-free connectivity and edge-clearance checks for round pads, linear tracks and round vias.
use crate::{
    board::NetId,
    geom::{Nm, Point, dist_seg_seg},
};
use std::collections::{BTreeMap, BTreeSet};
/// A capsule of copper spanning a continuous range of layer indices.
/// Coincident endpoints describe a disk.
#[derive(Clone, Copy, Debug)]
pub struct Conductor {
    /// Logical net.
    pub net: NetId,
    /// Segment endpoints.
    pub axis: [Point; 2],
    /// Full width, in nanometres.
    pub width: Nm,
    /// Inclusive low and high copper layer indices.
    pub layers: [u16; 2],
    /// True for a component pad.
    pub terminal: bool,
    /// Finished via hole, if present.
    pub drill: Option<Nm>,
}
/// A finding names input object indices or the open net.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finding {
    /// Two different nets touch.
    Short(usize, usize),
    /// The copper edges are closer than the minimum clearance.
    Clearance(usize, usize),
    /// Pads of this net lie on distinct as-built islands.
    Open(NetId),
    /// Finished drill is too small.
    Drill(usize),
    /// Radial annular ring is too small.
    Ring(usize),
}
fn root(parents: &mut [usize], mut i: usize) -> usize {
    while parents[i] != i {
        parents[i] = parents[parents[i]];
        i = parents[i];
    }
    i
}
/// Sweep by left copper edge. Auxiliary storage scales with objects and terminal nets.
/// A caller can defer open checks for nets using copper shapes outside the capsule model.
pub fn check(
    items: &[Conductor],
    clearance: Nm,
    min_drill: Nm,
    ring: Nm,
    deferred: &BTreeSet<NetId>,
    mut report: impl FnMut(Finding),
) -> Result<(), &'static str> {
    if [clearance, min_drill, ring].iter().any(|n| n.0 < 0) {
        return Err("negative rule");
    }
    for item in items {
        if item.width.0 <= 0
            || item.layers[0] > item.layers[1]
            || item.drill.is_some_and(|d| d.0 <= 0)
            || item
                .axis
                .iter()
                .any(|p| i32::try_from(p.x.0).is_err() || i32::try_from(p.y.0).is_err())
        {
            return Err("invalid copper dimension, coordinate or span");
        }
    }
    let extent = |i: usize| {
        let t = &items[i];
        let [a, b] = t.axis;
        (
            (a.x.0.min(b.x.0) as i128) * 2 - i128::from(t.width.0),
            (a.x.0.max(b.x.0) as i128) * 2 + i128::from(t.width.0),
        )
    };
    let mut order: Vec<_> = (0..items.len()).collect();
    order.sort_unstable_by_key(|&i| (extent(i).0, i));
    let mut parents: Vec<_> = (0..items.len()).collect();
    let mut active: Vec<usize> = Vec::new();
    for i in order {
        let a = &items[i];
        if let Some(d) = a.drill {
            if d < min_drill {
                report(Finding::Drill(i));
            }
            if i128::from(a.width.0) - i128::from(d.0) < 2 * i128::from(ring.0) {
                report(Finding::Ring(i));
            }
        }
        let left = extent(i).0 - 2 * i128::from(clearance.0);
        active.retain(|&j| extent(j).1 >= left);
        for &j in &active {
            let b = &items[j];
            if a.layers[0] > b.layers[1] || b.layers[0] > a.layers[1] {
                continue;
            }
            let gap = dist_seg_seg(a.axis[0], a.axis[1], b.axis[0], b.axis[1])
                - (a.width.0 as f64 + b.width.0 as f64) * 0.5;
            if a.net == b.net {
                if gap <= 0. {
                    let r = root(&mut parents, i);
                    let s = root(&mut parents, j);
                    parents[r] = s;
                }
            } else if gap <= 0. {
                report(Finding::Short(j, i));
            } else if gap < (clearance.0 as f64) {
                report(Finding::Clearance(j, i));
            }
        }
        active.push(i);
    }
    let mut first = BTreeMap::new();
    let mut open = BTreeSet::new();
    for (i, t) in items
        .iter()
        .enumerate()
        .filter(|(_, t)| t.terminal && !deferred.contains(&t.net))
    {
        let r = root(&mut parents, i);
        if *first.entry(t.net).or_insert(r) != r && open.insert(t.net) {
            report(Finding::Open(t.net));
        }
    }
    Ok(())
}
