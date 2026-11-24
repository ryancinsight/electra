use super::{Error, OperatingPoint, Si, SolveOptions, topology::Topology};
use crate::{Branch, sweep};
use aequitas::systems::si::quantities::Dimensionless;
use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};

/// A compiled passive network and reusable buffers for a series of independent solves.
/// Branches are borrowed. Fixed-node membership stays constant while prescribed
/// values and nodal injections can change. Each solve starts at the same initial
/// guess as the one-shot API, including after a failed solve.
pub struct PreparedNetwork<'a, P, G, F> {
    topology: Topology<'a, G>,
    point: OperatingPoint<P, F>,
    rhs: Vec<Si<F>>,
    scale: Vec<Si<F>>,
}

impl<'a, P, G, F> PreparedNetwork<'a, P, G, F>
where
    Si<G>: AddAssign + Mul<Si<P>, Output = Si<F>>,
    Si<P>: Sub<Output = Si<P>>,
    Si<F>: Add<Output = Si<F>>
        + AddAssign
        + SubAssign
        + Div<Si<G>, Output = Si<P>>
        + Mul<Dimensionless, Output = Si<F>>,
{
    /// Compile the connectivity, check anchoring, and allocate the solution buffers.
    pub fn new(fixed: &[Option<Si<P>>], branches: &'a [Branch<G>]) -> Result<Self, Error<F>> {
        let topology = Topology::new(fixed, branches)?;
        let n = fixed.len();
        Ok(Self {
            topology,
            point: OperatingPoint {
                potentials: vec![Si::default(); n],
                reactions: vec![Si::default(); n],
                max_residual: Si::default(),
                sweeps: 0,
            },
            rhs: vec![Si::default(); n],
            scale: vec![Si::default(); n],
        })
    }

    pub(super) fn into_point(self) -> OperatingPoint<P, F> {
        self.point
    }

    /// Solve without further allocation. The returned point borrows these buffers
    /// until the next mutable use. An invalid input or exhausted budget is retryable.
    pub fn solve(
        &mut self,
        fixed: &[Option<Si<P>>],
        injections: &[Si<F>],
        options: SolveOptions<F>,
    ) -> Result<&OperatingPoint<P, F>, Error<F>> {
        super::validate_inputs(fixed, injections, &options)?;
        if fixed.len() != self.topology.fixed.len()
            || fixed
                .iter()
                .zip(&self.topology.fixed)
                .any(|(q, &prescribed)| q.is_some() != prescribed)
        {
            return Err(Error::InvalidInput);
        }
        self.solve_validated(fixed, injections, options)
    }

    pub(super) fn solve_validated(
        &mut self,
        fixed: &[Option<Si<P>>],
        injections: &[Si<F>],
        options: SolveOptions<F>,
    ) -> Result<&OperatingPoint<P, F>, Error<F>> {
        let graph = &self.topology;
        let point = &mut self.point;
        for (out, &value) in point.potentials.iter_mut().zip(fixed) {
            *out = value.unwrap_or_default();
        }
        self.rhs.copy_from_slice(injections);
        for &(row, other, g) in &graph.boundary {
            self.rhs[row] += g * point.potentials[other];
        }
        if self.rhs.iter().any(|q| !q.as_base().is_finite()) {
            return Err(Error::Arithmetic);
        }
        for count in 1..=options.max_sweeps {
            sweep(&mut point.potentials, graph.equations(&self.rhs));
            for ((residual, scale), &q) in point
                .reactions
                .iter_mut()
                .zip(&mut self.scale)
                .zip(injections)
            {
                *residual = -q;
                *scale = q.abs();
            }
            for edge in graph.branches {
                let current =
                    edge.conductance * (point.potentials[edge.from] - point.potentials[edge.to]);
                point.reactions[edge.from] += current;
                point.reactions[edge.to] -= current;
                self.scale[edge.from] += current.abs();
                self.scale[edge.to] += current.abs();
            }
            let mut converged = true;
            point.max_residual = Si::default();
            for (i, imbalance) in point.reactions.iter_mut().enumerate() {
                if !imbalance.as_base().is_finite() || !self.scale[i].as_base().is_finite() {
                    return Err(Error::Arithmetic);
                }
                if !graph.fixed[i] {
                    let residual = imbalance.abs();
                    point.max_residual = point.max_residual.max(residual);
                    converged &= residual <= options.absolute + self.scale[i] * options.relative;
                    *imbalance = Si::default();
                }
            }
            if converged {
                point.sweeps = count;
                return Ok(point);
            }
        }
        Err(Error::NoConvergence {
            sweeps: options.max_sweeps,
            max_residual: point.max_residual,
        })
    }
}
