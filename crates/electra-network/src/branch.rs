use aequitas::Quantity;
use core::ops::{Mul, Neg};

/// An oriented two-terminal branch with a passive conductance.
/// Its orientation defines the sign of reported flux.
#[derive(Clone, Copy, Debug)]
pub struct Branch<G, T = f64> {
    /// First endpoint (positive flux leaves this node).
    pub from: usize,
    /// Second endpoint.
    pub to: usize,
    /// Flux per potential difference, in a consistent unit system.
    pub conductance: Quantity<T, G>,
}

impl<G, T: Copy> Branch<G, T> {
    /// Scatter the reduced nodal stamp, eliminating prescribed endpoints.
    /// The caller validates the branch and sets identity rows if it stores fixed nodes.
    /// The callbacks are statically dispatched and need no temporary matrix.
    pub fn stamp<P, F>(
        &self,
        fixed: [Option<Quantity<T, P>>; 2],
        mut matrix: impl FnMut(usize, usize, Quantity<T, G>),
        mut rhs: impl FnMut(usize, Quantity<T, F>),
    ) where
        Quantity<T, G>: Neg<Output = Quantity<T, G>> + Mul<Quantity<T, P>, Output = Quantity<T, F>>,
    {
        let g = self.conductance;
        for (i, j, here, there) in [
            (self.from, self.to, fixed[0], fixed[1]),
            (self.to, self.from, fixed[1], fixed[0]),
        ] {
            if here.is_none() {
                matrix(i, i, g);
                if let Some(value) = there {
                    rhs(i, g * value);
                } else {
                    matrix(i, j, -g);
                }
            }
        }
    }
}
