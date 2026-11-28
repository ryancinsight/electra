//! Pad-alignment and anchored placement experiment with local courtyard legalization.
use super::input::Input;
use kwavers_driver::{
    geom::{Nm, Point},
    place::{AnnealParams, PlaceConfig, anneal},
};

pub(crate) fn place(input: &mut Input, cfg: &PlaceConfig, movable: &[usize], seed: u64) {
    // A short driver anneal supplies the orientation search for movable parts.
    anneal(
        &mut input.parts,
        &input.library,
        cfg,
        movable,
        &AnnealParams {
            steps: 80,
            seed,
            ..Default::default()
        },
        None,
    );
    // Freeze each sweep's net barycentres so component ordering does not change the force.
    let mut centres = vec![(0., 0., 0u32); input.board.nets.len()];
    for _ in 0..3 {
        centres.fill((0., 0., 0));
        for c in &input.parts {
            for (p, _, net) in c.placed_pads(&input.library) {
                if let Some(n) = net {
                    let anchor = if c.locked { 4 } else { 1 };
                    let a = &mut centres[n.0 as usize];
                    a.0 += p.x.to_mm() * f64::from(anchor);
                    a.1 += p.y.to_mm() * f64::from(anchor);
                    a.2 += anchor;
                }
            }
        }
        for &idx in movable {
            let c = &input.parts[idx];
            let original = c.placement.pos;
            let mut dx = 0.;
            let mut dy = 0.;
            let mut count = 0;
            for (p, _, net) in c.placed_pads(&input.library) {
                if let Some(n) = net {
                    let (x, y, w) = centres[n.0 as usize];
                    dx += x / f64::from(w) - p.x.to_mm();
                    dy += y / f64::from(w) - p.y.to_mm();
                    count += 1;
                }
            }
            if count == 0 {
                continue;
            }
            let target = Point::new(
                original.x + Nm::from_mm(dx / (2. * f64::from(count))),
                original.y + Nm::from_mm(dy / (2. * f64::from(count))),
            );
            // Choose a legal location on a bounded window around the anchor target.
            let half = c.placement.rot.apply_size(input.library[c.fp].courtyard);
            let mut best = None;
            for ix in -3..=3 {
                for iy in -3..=3 {
                    let p = Point::new(
                        target.x + Nm::from_mm(f64::from(ix)),
                        target.y + Nm::from_mm(f64::from(iy)),
                    );
                    if p.x.0 - half.0.0 / 2 < cfg.margin.0
                        || p.y.0 - half.1.0 / 2 < cfg.margin.0
                        || p.x.0 + half.0.0 / 2 > cfg.board.0.0 - cfg.margin.0
                        || p.y.0 + half.1.0 / 2 > cfg.board.1.0 - cfg.margin.0
                    {
                        continue;
                    }
                    input.parts[idx].placement.pos = p;
                    let rect = input.parts[idx]
                        .courtyard(&input.library)
                        .inflate(cfg.courtyard_clearance);
                    if input.parts.iter().enumerate().any(|(j, c)| {
                        j != idx && rect.overlap_area(c.courtyard(&input.library)) > 0.
                    }) {
                        continue;
                    }
                    let score = p.euclid(target);
                    if best.is_none_or(|(old, _)| score < old) {
                        best = Some((score, p));
                    }
                }
            }
            input.parts[idx].placement.pos = best.map_or(original, |(_, p)| p);
        }
    }
}
