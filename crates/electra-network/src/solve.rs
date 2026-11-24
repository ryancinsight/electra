use crate::{Branch, RelaxationRow, sweep};
use std::fmt;

/// Limits for a double-precision operating point.
#[derive(Copy, Clone, Debug)]
pub struct SolveOptions {
    /// Absolute nodal conservation tolerance in flow or current units.
    pub absolute: f64,
    /// Relative tolerance against the sum of absolute physical flows at each node.
    pub relative: f64,
    /// Maximum number of Gauss–Seidel sweeps.
    pub max_sweeps: usize,
}
impl Default for SolveOptions {
    fn default() -> Self {
        Self {
            absolute: 1e-12,
            relative: 1e-10,
            max_sweeps: 100_000,
        }
    }
}

/// A solution that passed conservation at every free node.
#[derive(Debug)]
pub struct OperatingPoint {
    /// Potential at each node in caller order.
    pub potentials: Vec<f64>,
    /// Supply injection needed at each fixed node in addition to specified injections.
    /// Entries for free nodes are zero.
    pub reactions: Vec<f64>,
    /// Largest absolute imbalance at a free node.
    pub max_residual: f64,
    /// Number of relaxation sweeps performed.
    pub sweeps: usize,
}

/// Failure to define, represent or converge a network.
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
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
        max_residual: f64,
    },
}
impl fmt::Display for Error {
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
                "no convergence after {sweeps} sweeps (maximum imbalance {max_residual:e})"
            ),
        }
    }
}
impl std::error::Error for Error {}

#[derive(Default)]
struct Row {
    diagonal: f64,
    neighbours: Vec<(usize, f64)>,
}

/// Solve a passive, linear network with prescribed potentials and signed nodal injections.
/// Every connected component must meet a prescribed node. Branches may be parallel.
/// Conservation is checked using physical branch fluxes after every sweep.
pub fn solve(
    fixed: &[Option<f64>],
    injections: &[f64],
    branches: &[Branch<f64>],
    options: SolveOptions,
) -> Result<OperatingPoint, Error> {
    let n = fixed.len();
    if n == 0
        || injections.len() != n
        || fixed
            .iter()
            .flatten()
            .chain(injections)
            .any(|x| !x.is_finite())
        || !options.absolute.is_finite()
        || !options.relative.is_finite()
        || options.absolute < 0.0
        || !(0.0..1.0).contains(&options.relative)
        || options.max_sweeps == 0
    {
        return Err(Error::InvalidInput);
    }
    let mut rows: Vec<Row> = (0..n).map(|_| Row::default()).collect();
    let mut rhs = injections.to_vec();
    let mut anchored: Vec<bool> = fixed.iter().map(Option::is_some).collect();
    for (i, &branch) in branches.iter().enumerate() {
        let (a, b) = (branch.from, branch.to);
        if a >= n
            || b >= n
            || a == b
            || !branch.conductance.is_finite()
            || branch.conductance <= 0.0
        {
            return Err(Error::InvalidBranch(i));
        }
        anchored[a] |= fixed[b].is_some();
        anchored[b] |= fixed[a].is_some();
        branch.stamp(
            [fixed[a], fixed[b]],
            |r, c, g| {
                if r == c {
                    rows[r].diagonal += g;
                } else {
                    rows[r].neighbours.push((c, -g));
                }
            },
            |r, x| rhs[r] += x,
        );
    }
    let mut stack: Vec<_> = (0..n).filter(|&i| anchored[i]).collect();
    while let Some(i) = stack.pop() {
        for &(j, _) in &rows[i].neighbours {
            if !anchored[j] {
                anchored[j] = true;
                stack.push(j);
            }
        }
    }
    if let Some(i) = anchored.iter().position(|v| !v) {
        return Err(Error::Floating(i));
    }
    if rows.iter().any(|row| !row.diagonal.is_finite()) || rhs.iter().any(|x| !x.is_finite()) {
        return Err(Error::Arithmetic);
    }
    let mut potentials: Vec<_> = fixed.iter().map(|v| v.unwrap_or_default()).collect();
    let mut imbalance = vec![0.0; n];
    let mut scale = vec![0.0; n];
    let mut max_residual = 0.0_f64;
    for sweeps in 1..=options.max_sweeps {
        sweep(
            &mut potentials,
            rows.iter()
                .enumerate()
                .filter(|(i, _)| fixed[*i].is_none())
                .map(|(i, row)| RelaxationRow {
                    node: i,
                    source: rhs[i],
                    diagonal: row.diagonal,
                    neighbours: row.neighbours.iter().copied(),
                }),
        );
        for (i, &q) in injections.iter().enumerate() {
            imbalance[i] = -q;
            scale[i] = q.abs();
        }
        for edge in branches {
            let flow = edge.conductance * (potentials[edge.from] - potentials[edge.to]);
            imbalance[edge.from] += flow;
            imbalance[edge.to] -= flow;
            scale[edge.from] += flow.abs();
            scale[edge.to] += flow.abs();
        }
        let mut converged = true;
        max_residual = 0.0;
        for i in 0..n {
            if !imbalance[i].is_finite() || !scale[i].is_finite() {
                return Err(Error::Arithmetic);
            }
            if fixed[i].is_none() {
                let r = imbalance[i].abs();
                max_residual = max_residual.max(r);
                converged &= r <= options.absolute + options.relative * scale[i];
                imbalance[i] = 0.0;
            }
        }
        if converged {
            return Ok(OperatingPoint {
                potentials,
                reactions: imbalance,
                max_residual,
                sweeps,
            });
        }
    }
    Err(Error::NoConvergence {
        sweeps: options.max_sweeps,
        max_residual,
    })
}
