use crate::{Length, MagneticPermeability, Permittivity};
use std::f64::consts::TAU;

/// External quasi-static TEM coefficients of a uniform homogeneous coaxial section.
/// The SI dimensions F/m and H/m coincide with permittivity and permeability.
#[derive(Clone, Copy, Debug)]
pub struct CoaxSection {
    /// Charge per unit length for one volt between the conductors.
    pub capacitance_per_length: Permittivity,
    /// External inductance per unit length, excluding conductor penetration.
    pub inductance_per_length: MagneticPermeability,
}
impl CoaxSection {
    /// Solve radial flux continuity between equipotential conductor surfaces.
    /// Each annular shell uses its midpoint face area. No mesh-sized storage is needed.
    pub fn extract(
        inner: Length,
        outer: Length,
        epsilon: Permittivity,
        mu: MagneticPermeability,
        shells: u32,
    ) -> Option<Self> {
        let (a, b, e, m) = (
            inner.into_base(),
            outer.into_base(),
            epsilon.into_base(),
            mu.into_base(),
        );
        if ![a, b, e, m].iter().all(|x| x.is_finite() && *x > 0.) || b <= a || shells == 0 {
            return None;
        }
        let dr = (b - a) / f64::from(shells);
        let integral: f64 = (0..shells)
            .map(|i| dr / (a + (f64::from(i) + 0.5) * dr))
            .sum();
        let c = TAU * e / integral;
        let l = m * integral / TAU;
        (c.is_finite() && c > 0. && l.is_finite() && l > 0.).then(|| Self {
            capacitance_per_length: Permittivity::from_base(c),
            inductance_per_length: MagneticPermeability::from_base(l),
        })
    }
}
