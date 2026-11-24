mod topology;
mod types;
mod workspace;

use crate::Branch;
use aequitas::systems::si::quantities::Dimensionless;
use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};
use types::Si;
pub use types::{Error, OperatingPoint, SolveOptions};
pub use workspace::PreparedNetwork;

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
    validate_inputs(fixed, injections, &options)?;
    let mut network = PreparedNetwork::new(fixed, branches)?;
    network.solve_validated(fixed, injections, options)?;
    Ok(network.into_point())
}

fn validate_inputs<P, F>(
    fixed: &[Option<Si<P>>],
    injections: &[Si<F>],
    options: &SolveOptions<F>,
) -> Result<(), Error<F>> {
    if fixed.is_empty()
        || injections.len() != fixed.len()
        || fixed.iter().flatten().any(|q| !q.as_base().is_finite())
        || injections.iter().any(|q| !q.as_base().is_finite())
        || !options.absolute.as_base().is_finite()
        || options.absolute < Si::default()
        || !(0.0..1.0).contains(options.relative.as_base())
        || options.max_sweeps == 0
    {
        return Err(Error::InvalidInput);
    }
    Ok(())
}
