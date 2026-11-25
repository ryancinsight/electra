//! Scaled modified nodal analysis with matrix-free iterative and explicit direct solves.
pub mod direct;
mod error;
mod linear;
mod result;
mod sweep;
mod system;
use crate::analysis::excitation::Excitation;
use crate::{Circuit, Dimensionless, ElectricCurrent, ElectricPotential, SolveOptions};
use athena_core::LinearOperator;
pub use error::MnaError;
use linear::Krylov;
use result::Balance;
pub use result::MnaPoint;
pub use sweep::{MnaSample, MnaSweep};
use system::{Drive, System};

/// Physical residual tolerances and Krylov iteration budget.
/// ```compile_fail
/// use electra_circuit::{ElectricCurrent, MnaOptions};
/// let _ = MnaOptions { voltage_absolute: ElectricCurrent::from_base(1e-12), ..Default::default() };
/// ```
#[derive(Clone, Copy, Debug)]
pub struct MnaOptions {
    /// Absolute nodal current imbalance in amperes.
    pub current_absolute: ElectricCurrent,
    /// Absolute ideal-source voltage discrepancy in volts.
    pub voltage_absolute: ElectricPotential,
    /// Relative tolerance for incident branch currents and individual source voltages.
    pub relative: Dimensionless,
    /// Maximum Arnoldi steps per point.
    pub max_iterations: usize,
}
impl Default for MnaOptions {
    fn default() -> Self {
        let dc = SolveOptions::default();
        Self {
            current_absolute: dc.absolute,
            voltage_absolute: ElectricPotential::from_base(1e-12),
            relative: dc.relative,
            max_iterations: dc.max_sweeps,
        }
    }
}
impl MnaOptions {
    fn validate(self) -> Result<(), MnaError> {
        if !self.voltage_absolute.as_base().is_finite()
            || self.voltage_absolute < ElectricPotential::default()
            || (SolveOptions {
                absolute: self.current_absolute,
                relative: self.relative,
                max_sweeps: self.max_iterations,
            })
            .validate()
            .is_err()
        {
            return Err(MnaError::InvalidOptions);
        }
        Ok(())
    }
}
struct Session<'a> {
    circuit: &'a Circuit,
    system: System,
    linear: Krylov,
    drive: Drive,
    balance: Vec<Balance>,
    point: MnaPoint<'a>,
    limits: MnaOptions,
}
impl<'a> Session<'a> {
    fn new(c: &'a Circuit, limits: MnaOptions, swept: Option<usize>) -> Result<Self, MnaError> {
        limits.validate()?;
        let system = System::new(c, usize::MAX)?;
        let mut linear = Krylov::new(system.dimension())?;
        let drive = system.rhs(c, swept, &mut linear.rhs)?;
        Ok(Self {
            circuit: c,
            system,
            linear,
            drive,
            balance: vec![Balance::default(); c.names.len()],
            point: MnaPoint::new(c),
            limits,
        })
    }
    fn run(&mut self, excitation: Excitation) -> Result<(), MnaError> {
        if let Some((_, x)) = excitation {
            self.drive.update(x, &mut self.linear.rhs)?;
        }
        let budget = self.limits.max_iterations;
        self.point.iterations = 0;
        let result = self.solve_candidate(excitation, budget, false);
        let remaining = budget - self.point.iterations;
        if remaining == 0 {
            return result;
        }
        match result {
            // A globally small residual can still mask a failing local equation.
            // Solve the correction at the scale of its own right-hand side.
            Err(
                MnaError::SourceVoltage { .. }
                | MnaError::CurrentBalance { .. }
                | MnaError::Resolution { .. },
            ) => self.solve_candidate(excitation, remaining, true),
            Err(MnaError::Krylov { .. })
                if excitation.is_some() && self.linear.x.iter().any(|&v| v != 0.) =>
            {
                self.linear.x.fill(0.);
                self.solve_candidate(excitation, remaining, false)
            }
            other => other,
        }
    }
    fn solve_candidate(
        &mut self,
        excitation: Excitation,
        budget: usize,
        refine: bool,
    ) -> Result<(), MnaError> {
        // A global dimensionless norm selects a candidate. Acceptance uses every
        // original KCL and KVL equation in physical units, including the reference.
        let target = self
            .limits
            .relative
            .into_base()
            .clamp(8. * f64::EPSILON, 1e-13);
        let report = self.linear.solve(&self.system, budget, target, refine)?;
        self.point.iterations += report.iterations;
        if !report.converged() {
            return Err(MnaError::Krylov {
                reason: report.termination,
                iterations: self.point.iterations,
            });
        }
        self.point.project(
            self.circuit,
            &self.system,
            &self.linear.x,
            excitation,
            self.limits,
            &mut self.balance,
        )
    }
}
impl Circuit {
    /// Solve a full modified nodal system for non-reference voltages and all voltage-defined branch currents.
    pub fn operating_point_mna(&self, limits: MnaOptions) -> Result<MnaPoint<'_>, MnaError> {
        let mut session = Session::new(self, limits, None)?;
        session.run(None)?;
        Ok(session.point)
    }
}
