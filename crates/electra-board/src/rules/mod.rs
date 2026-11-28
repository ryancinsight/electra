//! Routed-length and via manufacturing margins. Positive values are inside the limit.
pub mod contact;
use crate::{
    board::{NetId, Track},
    geom::Nm,
};
/// Capture and layout share the identifier independently of the constraint's display name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstraintId(pub u64);
/// Maximum routed length per conductor and maximum absolute mismatch.
#[derive(Clone, Copy, Debug)]
pub struct PairRule {
    /// Persistent constraint key.
    pub id: ConstraintId,
    /// Positive and negative board net identifiers.
    pub nets: [NetId; 2],
    /// Maximum planar copper length of either member.
    pub length: Nm,
    /// Maximum absolute length difference.
    pub skew: Nm,
}
/// Measured centreline lengths and signed margins in nanometres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PairMargins {
    /// Positive then negative member lengths.
    pub lengths: [f64; 2],
    /// Maximum allowed length minus the longer measured member.
    pub length: f64,
    /// Allowed skew minus the absolute length difference.
    pub skew: f64,
}
/// Accumulated centreline lengths for exactly one rule.
pub struct PairSweep<'a> {
    rule: &'a PairRule,
    lengths: [f64; 2],
}
impl PairRule {
    /// Validate the rule once and start a constant-storage scan.
    pub fn sweep(&self) -> Result<PairSweep<'_>, &'static str> {
        if self.nets[0] == self.nets[1] || self.length.0 < 0 || self.skew.0 < 0 {
            return Err("invalid pair rule");
        }
        Ok(PairSweep {
            rule: self,
            lengths: [0.; 2],
        })
    }
    /// Measure the existing copper in its iteration order.
    pub fn evaluate<'a>(
        &self,
        tracks: impl IntoIterator<Item = &'a Track>,
    ) -> Result<PairMargins, &'static str> {
        let mut scan = self.sweep()?;
        for t in tracks {
            scan.segment(t.net, t.start, t.end);
        }
        scan.finish()
    }
}
impl PairSweep<'_> {
    /// Add one routed segment. Segments on unrelated nets have no effect.
    pub fn segment(&mut self, net: NetId, a: crate::geom::Point, b: crate::geom::Point) {
        if let Some(i) = self.rule.nets.iter().position(|n| *n == net) {
            self.lengths[i] += a.euclid(b);
        }
    }
    /// Return signed margins. An unrouted conductor is an error.
    pub fn finish(self) -> Result<PairMargins, &'static str> {
        let lengths = self.lengths;
        if lengths.contains(&0.) {
            return Err("unrouted pair member");
        }
        Ok(PairMargins {
            lengths,
            length: self.rule.length.0 as f64 - lengths[0].max(lengths[1]),
            skew: self.rule.skew.0 as f64 - (lengths[0] - lengths[1]).abs(),
        })
    }
}
