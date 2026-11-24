use core::ops::{Mul, Neg};

/// An oriented two-terminal branch with a passive conductance.
/// Its orientation defines the sign of reported flux.
#[derive(Clone, Copy, Debug)]
pub struct Branch<T> {
    /// First endpoint (positive flux leaves this node).
    pub from: usize,
    /// Second endpoint.
    pub to: usize,
    /// Flux per potential difference, in a consistent unit system.
    pub conductance: T,
}

impl<T: Copy + Mul<Output = T> + Neg<Output = T>> Branch<T> {
    /// Scatter the reduced nodal stamp, eliminating prescribed endpoints.
    /// The caller validates the branch and sets identity rows if it stores fixed nodes.
    /// The callbacks are statically dispatched and need no temporary matrix.
    pub fn stamp(
        self,
        fixed: [Option<T>; 2],
        mut matrix: impl FnMut(usize, usize, T),
        mut rhs: impl FnMut(usize, T),
    ) {
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
