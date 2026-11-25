//! Provider interface for checked direct solutions of normalized MNA systems.
mod session;
mod sweep;

use super::{MnaError, MnaOptions, MnaPoint, System};
use crate::Circuit;
use athena_core::LinearOperator;
use session::Session;
pub use sweep::DirectSweep;

/// Borrowed scaled coefficient matrix with the reference equation and column eliminated.
/// The first unknowns are node potentials, followed by voltage-defined branch currents.
#[derive(Clone, Copy)]
pub struct Matrix<'a>(&'a System);
impl Matrix<'_> {
    /// Number of normalized unknowns and equations.
    pub fn order(self) -> usize {
        self.0.dimension() - 1
    }

    /// Visit additive stamp contributions with zero-based indices. Multiple contributions
    /// may share a coordinate or cancel. Accumulate them before factorization.
    pub fn for_each_entry(self, mut visit: impl FnMut(usize, usize, f64)) {
        self.0.coefficients(|r, c, v| visit(r - 1, c - 1, v));
    }
}

/// Static factorization provider. Electra performs the physical checks on every result.
pub trait DirectFactor: Sized {
    /// Largest supported order, enforced before assembling the normalized operator.
    const MAX_ORDER: usize;
    /// Factor once for this circuit topology and its fixed linear element values.
    fn factor(matrix: Matrix<'_>) -> Result<Self, MnaError>;
    /// Overwrite every normalized unknown using the supplied right-hand side.
    fn solve_into(&mut self, rhs: &[f64], solution: &mut [f64]) -> Result<(), MnaError>;
}

impl Circuit {
    /// Compute a DC operating point using a caller-selected, statically bound direct factor.
    pub fn operating_point_direct<F: DirectFactor>(
        &self,
        limits: MnaOptions,
    ) -> Result<MnaPoint<'_>, MnaError> {
        let mut session = Session::<F>::new(self, limits, None)?;
        session.run(None)?;
        Ok(session.point)
    }
}
