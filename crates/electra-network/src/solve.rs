use crate::{Branch, RelaxationRow, sweep};
use aequitas::{Quantity, systems::si::quantities::Dimensionless};
use std::{
    fmt,
    ops::{Add, AddAssign, Div, Mul, Sub, SubAssign},
};

type Si<D> = Quantity<f64, D>;

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

struct Row<G> {
    diagonal: Si<G>,
    neighbours: Vec<(usize, Si<G>)>,
}

/// Solve a passive, linear network with prescribed potentials and signed nodal injections.
/// Every connected component must meet a prescribed node. Branches may be parallel.
/// Conservation is checked using physical branch fluxes after every sweep.
/// The conductance multiplied by a potential must have the flux dimension.
/// A hydraulic branch cannot be connected to an electrical nodal equation.
/// ```compile_fail
/// use aequitas::systems::si::quantities::{ElectricCurrent, ElectricPotential, HydraulicConductance};
/// use electra_network::{Branch, solve};
/// let hydraulic = Branch { from: 0, to: 1, conductance: HydraulicConductance::from_base(1.0) };
/// let _ = solve(&[Some(ElectricPotential::default()), None], &[ElectricCurrent::default(); 2],
///               &[hydraulic], Default::default());
/// ```
pub fn solve<P, G, F>(
    fixed: &[Option<Si<P>>],
    injections: &[Si<F>],
    branches: &[Branch<G>],
    options: SolveOptions<F>,
) -> Result<OperatingPoint<P, F>, Error<F>>
where
    Si<G>: AddAssign + Mul<Si<P>, Output = Si<F>>,
    Si<P>: Sub<Output = Si<P>>,
    Si<F>: Add<Output = Si<F>>
        + AddAssign
        + SubAssign
        + Div<Si<G>, Output = Si<P>>
        + Mul<Dimensionless, Output = Si<F>>,
{
    let n = fixed.len();
    if n == 0
        || injections.len() != n
        || fixed.iter().flatten().any(|x| !x.as_base().is_finite())
        || injections.iter().any(|q| !q.as_base().is_finite())
        || !options.absolute.as_base().is_finite()
        || !options.relative.as_base().is_finite()
        || options.absolute < Quantity::default()
        || !(0.0..1.0).contains(options.relative.as_base())
        || options.max_sweeps == 0
    {
        return Err(Error::InvalidInput);
    }
    let mut rows: Vec<Row<G>> = (0..n)
        .map(|_| Row {
            diagonal: Quantity::default(),
            neighbours: Vec::new(),
        })
        .collect();
    let mut rhs = injections.to_vec();
    let mut anchored: Vec<bool> = fixed.iter().map(Option::is_some).collect();
    for (i, branch) in branches.iter().enumerate() {
        let (a, b) = (branch.from, branch.to);
        if a >= n
            || b >= n
            || a == b
            || !branch.conductance.as_base().is_finite()
            || branch.conductance <= Quantity::default()
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
    if rows.iter().any(|row| !row.diagonal.as_base().is_finite())
        || rhs.iter().any(|x| !x.as_base().is_finite())
    {
        return Err(Error::Arithmetic);
    }
    let mut potentials: Vec<_> = fixed.iter().map(|v| v.unwrap_or_default()).collect();
    let mut imbalance = vec![Si::<F>::default(); n];
    let mut scale = vec![Si::<F>::default(); n];
    let mut max_residual = Si::<F>::default();
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
        max_residual = Si::<F>::default();
        for i in 0..n {
            if !imbalance[i].as_base().is_finite() || !scale[i].as_base().is_finite() {
                return Err(Error::Arithmetic);
            }
            if fixed[i].is_none() {
                let r = imbalance[i].abs();
                max_residual = max_residual.max(r);
                converged &= r <= options.absolute + scale[i] * options.relative;
                imbalance[i] = Quantity::default();
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
