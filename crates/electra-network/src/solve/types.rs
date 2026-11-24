use aequitas::{Quantity, systems::si::quantities::Dimensionless};
use std::fmt;

pub(super) type Si<D> = Quantity<f64, D>;

/// Limits for a double-precision operating point.
#[derive(Copy, Clone, Debug)]
pub struct SolveOptions<F> {
    /// Absolute nodal conservation tolerance in flow or current units.
    pub absolute: Si<F>,
    /// Relative tolerance against the sum of absolute physical flows at each node.
    pub relative: Dimensionless,
    /// Maximum number of Gauss–Seidel sweeps.
    pub max_sweeps: usize,
}
impl<F> Default for SolveOptions<F> {
    fn default() -> Self {
        Self {
            absolute: Quantity::from_base(1e-12),
            relative: Dimensionless::from_base(1e-10),
            max_sweeps: 100_000,
        }
    }
}

/// A solution that passed conservation at every free node.
#[derive(Debug)]
pub struct OperatingPoint<P, F> {
    /// Potential at each node in caller order.
    pub potentials: Vec<Si<P>>,
    /// Supply injection needed at each fixed node in addition to specified injections.
    /// Entries for free nodes are zero.
    pub reactions: Vec<Si<F>>,
    /// Largest absolute imbalance at a free node.
    pub max_residual: Si<F>,
    /// Number of relaxation sweeps performed.
    pub sweeps: usize,
}

/// Failure to define, represent or converge a network.
#[derive(Clone, Debug, PartialEq)]
pub enum Error<F> {
    /// Input lengths, solver options, or prescribed values are invalid.
    InvalidInput,
    /// Invalid endpoints or conductance; index is in caller order.
    InvalidBranch(usize),
    /// This component has no prescribed potential.
    Floating(usize),
    /// Arithmetic exceeded the finite representable range.
    Arithmetic,
    /// Conservation tolerance failed at the iteration budget.
    NoConvergence {
        /// Number of sweeps executed.
        sweeps: usize,
        /// Worst absolute imbalance in caller units.
        max_residual: Si<F>,
    },
}
impl<F> fmt::Display for Error<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("invalid network inputs or solver options"),
            Self::InvalidBranch(i) => write!(f, "invalid branch at index {i}"),
            Self::Floating(i) => write!(f, "floating component containing node {i}"),
            Self::Arithmetic => f.write_str("non-finite arithmetic in network solution"),
            Self::NoConvergence {
                sweeps,
                max_residual,
            } => write!(
                f,
                "no convergence after {sweeps} sweeps (maximum imbalance {:e} SI)",
                max_residual.as_base()
            ),
        }
    }
}
impl<F: fmt::Debug> std::error::Error for Error<F> {}
