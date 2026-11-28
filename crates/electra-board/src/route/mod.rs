//! Routed copper measurements.
use crate::{
    board::{LayerId, Track},
    geom::{Nm, Point},
};
use std::collections::BTreeMap;
/// Midpoint of a track segment.
pub fn track_midpoint(track: &Track) -> Point {
    Point::new(
        Nm(((i128::from(track.start.x.0) + i128::from(track.end.x.0)) / 2) as i64),
        Nm(((i128::from(track.start.y.0) + i128::from(track.end.y.0)) / 2) as i64),
    )
}

/// Cumulative routed length per layer for a set of tracks.
pub fn diff_pair_layer_segment_lengths(
    tracks: &[&Track],
) -> BTreeMap<LayerId, (f64, Option<Point>)> {
    let mut lengths = BTreeMap::new();
    for track in tracks {
        let entry = lengths
            .entry(track.layer)
            .or_insert_with(|| (0.0, Some(track_midpoint(track))));
        entry.0 += track.start.euclid(track.end);
    }
    lengths
}
