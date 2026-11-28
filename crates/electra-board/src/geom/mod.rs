//! Integer nanometre geometry and planar distance queries.
mod length;
pub use length::Nm;
/// A point on the board in exact nanometre coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Point {
    /// X coordinate.
    pub x: Nm,
    /// Y coordinate.
    pub y: Nm,
}

impl Point {
    /// Construct a point from two nanometre coordinates.
    #[inline]
    #[must_use]
    pub fn new(x: Nm, y: Nm) -> Self {
        Point { x, y }
    }

    /// Manhattan (L1) distance to another point, in nanometres.
    #[inline]
    #[must_use]
    pub fn manhattan(self, other: Point) -> Nm {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    /// Euclidean distance to another point, in nanometres (as `f64`).
    #[inline]
    #[must_use]
    pub fn euclid(self, other: Point) -> f64 {
        let dx = (i128::from(self.x.0) - i128::from(other.x.0)) as f64;
        let dy = (i128::from(self.y.0) - i128::from(other.y.0)) as f64;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Signed orientation of the triplet `(a, b, c)` (twice the signed triangle area): `>0` left turn,
/// `<0` right turn, `0` collinear. Exact integer arithmetic (nm coordinates fit in `i128`).
#[inline]
#[must_use]
pub fn orient(a: Point, b: Point, c: Point) -> i128 {
    let (ax, ay) = (a.x.0 as i128, a.y.0 as i128);
    let (bx, by) = (b.x.0 as i128, b.y.0 as i128);
    let (cx, cy) = (c.x.0 as i128, c.y.0 as i128);
    (bx - ax) * (cy - ay) - (by - ay) * (cx - ax)
}

/// Whether segments `p1p2` and `p3p4` *properly* cross (intersect at an interior point of both).
/// Shared endpoints and collinear overlap are not counted — this measures genuine lane crossings.
#[inline]
#[must_use]
pub fn segments_cross(p1: Point, p2: Point, p3: Point, p4: Point) -> bool {
    let d1 = orient(p3, p4, p1);
    let d2 = orient(p3, p4, p2);
    let d3 = orient(p1, p2, p3);
    let d4 = orient(p1, p2, p4);
    ((d1 > 0 && d2 < 0) || (d1 < 0 && d2 > 0)) && ((d3 > 0 && d4 < 0) || (d3 < 0 && d4 > 0))
}

/// Euclidean distance (nm, as `f64`) from point `p` to segment `ab` — the closest approach, with
/// the foot clamped to the segment endpoints. Degenerate (zero-length) `ab` reduces to point distance.
#[inline]
#[must_use]
pub fn dist_point_seg(p: Point, a: Point, b: Point) -> f64 {
    let (ax, ay) = (a.x.0 as f64, a.y.0 as f64);
    let (bx, by) = (b.x.0 as f64, b.y.0 as f64);
    let (px, py) = (p.x.0 as f64, p.y.0 as f64);
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        return p.euclid(a);
    }
    let t = (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0);
    let (cx, cy) = (ax + t * dx, ay + t * dy);
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}

/// Minimum Euclidean distance (nm, as `f64`) between segments `ab` and `cd`. Zero if they cross or
/// touch; otherwise the smallest of the four endpoint-to-opposite-segment distances (exact for the
/// non-crossing case, since the closest pair then involves at least one endpoint).
#[inline]
#[must_use]
pub fn dist_seg_seg(a: Point, b: Point, c: Point, d: Point) -> f64 {
    if segments_cross(a, b, c, d) {
        return 0.0;
    }
    dist_point_seg(a, c, d)
        .min(dist_point_seg(b, c, d))
        .min(dist_point_seg(c, a, b))
        .min(dist_point_seg(d, a, b))
}

/// Whether `p` lies inside `poly` using the even-odd rule. Boundary points are not treated as
/// inside; callers that need boundary-inclusive behavior should combine this with segment-distance
/// checks using the required tolerance.
#[must_use]
pub fn point_in_polygon(p: Point, poly: &[Point]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let pi = poly[i];
        let pj = poly[j];
        if pi.y.0 == pj.y.0 {
            j = i;
            continue;
        }
        let edge_x = (pj.x.0 - pi.x.0) as f64 * (p.y.0 - pi.y.0) as f64 / (pj.y.0 - pi.y.0) as f64
            + pi.x.0 as f64;
        if ((pi.y.0 > p.y.0) != (pj.y.0 > p.y.0)) && ((p.x.0 as f64) < edge_x) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Minimum distance from `p` to the closed polygon boundary. Returns `None` for a degenerate
/// polygon with fewer than two points.
#[must_use]
pub fn distance_to_polygon_boundary(p: Point, poly: &[Point]) -> Option<f64> {
    if poly.len() < 2 {
        return None;
    }
    let mut best = f64::INFINITY;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        best = best.min(dist_point_seg(p, a, b));
    }
    Some(best)
}

/// Convex hull of a point set (Andrew's monotone chain), returned counter-clockwise without the
/// duplicated closing point. Fewer than 3 unique points returns the input sorted (degenerate hull).
/// Exact integer orientation, so collinear points on an edge are dropped.
#[must_use]
pub fn convex_hull(points: &[Point]) -> Vec<Point> {
    let mut pts: Vec<Point> = points.to_vec();
    pts.sort_unstable_by_key(|p| (p.x.0, p.y.0));
    pts.dedup_by(|a, b| a.x.0 == b.x.0 && a.y.0 == b.y.0);
    let n = pts.len();
    if n < 3 {
        return pts;
    }
    let mut hull: Vec<Point> = Vec::with_capacity(2 * n);
    // Lower hull, then upper hull; a right turn (orient <= 0) pops the middle point.
    for &p in pts.iter() {
        while hull.len() >= 2 && orient(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0 {
            hull.pop();
        }
        hull.push(p);
    }
    let lower = hull.len() + 1;
    for &p in pts.iter().rev() {
        while hull.len() >= lower && orient(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0 {
            hull.pop();
        }
        hull.push(p);
    }
    hull.pop(); // last point == first
    hull
}
