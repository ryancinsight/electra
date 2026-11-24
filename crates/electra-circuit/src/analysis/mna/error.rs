use crate::{ElectricCurrent, ElectricPotential, SourceValue, SweepError};
use athena_core::Termination;
use std::fmt;

/// Failure to prepare or qualify a modified nodal operating point.
#[derive(Debug)]
pub enum MnaError {
    /// Invalid physical tolerances or iteration limit.
    InvalidOptions,
    /// An unanchored passive or voltage-source component.
    Floating(usize),
    /// Unrepresentable physical or normalized result.
    Arithmetic,
    /// Storage allocation or provider contract failure.
    Backend(&'static str),
    /// Athena terminated before numerical convergence.
    Krylov {
        /// Provider termination.
        reason: Termination,
        /// Completed Arnoldi steps.
        iterations: usize,
    },
    /// Physical KCL acceptance failed at an original node.
    CurrentBalance {
        /// Circuit node index.
        node: usize,
        /// Absolute measured current imbalance.
        residual: ElectricCurrent,
        /// Per-node allowed imbalance.
        limit: ElectricCurrent,
    },
    /// A measured source drop failed its physical voltage equation.
    SourceVoltage {
        /// Original part index.
        part: usize,
        /// Absolute measured discrepancy.
        residual: ElectricPotential,
        /// Allowed discrepancy.
        limit: ElectricPotential,
    },
    /// A nonzero ideal voltage collapsed to zero in the returned node potentials.
    Resolution {
        /// Original part index.
        part: usize,
    },
    /// Sweep definition or grid error.
    Sweep(SweepError),
    /// Failure at an identified source excitation.
    Point(SourceValue, Box<MnaError>),
}
impl fmt::Display for MnaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOptions => f.write_str("invalid MNA tolerances or iteration budget"),
            Self::Floating(n) => write!(f, "floating component containing node {n}"),
            Self::Arithmetic => f.write_str("non-finite or unrepresentable MNA result"),
            Self::Backend(e) => f.write_str(e),
            Self::Krylov { reason, iterations } => {
                write!(f, "Athena GMRES {reason:?} after {iterations} iterations")
            }
            Self::CurrentBalance {
                node,
                residual,
                limit,
            } => write!(
                f,
                "node {node} KCL residual {} A exceeds {} A",
                residual.as_base(),
                limit.as_base()
            ),
            Self::SourceVoltage {
                part,
                residual,
                limit,
            } => write!(
                f,
                "source at part {part} has voltage residual {} V exceeding {} V",
                residual.as_base(),
                limit.as_base()
            ),
            Self::Resolution { part } => write!(
                f,
                "nonzero voltage at part {part} cannot be resolved in node potentials"
            ),
            Self::Sweep(e) => e.fmt(f),
            Self::Point(q, e) => write!(f, "MNA sweep failed at {q}: {e}"),
        }
    }
}
impl std::error::Error for MnaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sweep(e) => Some(e),
            Self::Point(_, e) => Some(e),
            _ => None,
        }
    }
}
