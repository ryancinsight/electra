//! Athena owns the recurrence. The scaled circuit operator borrows host slice views.
mod vectors;
use super::MnaError;
use athena_core::{
    ConvergencePolicy, Gmres, GmresWorkspace, Identity, LinearOperator, SolveReport,
};
pub(super) use vectors::Host;

// Fixed restart width bounds basis storage independently of iteration budget.
const RESTART: usize = 16;
pub(super) struct Krylov {
    workspace: GmresWorkspace<Host, RESTART>,
    refinement: Option<(Vec<f64>, Vec<f64>)>,
    pub x: Vec<f64>,
    pub rhs: Vec<f64>,
}
impl Krylov {
    pub fn new(n: usize) -> Result<Self, MnaError> {
        Ok(Self {
            workspace: GmresWorkspace::new(&Host, n).map_err(MnaError::Backend)?,
            refinement: None,
            x: vec![0.; n],
            rhs: vec![0.; n],
        })
    }
    pub fn solve(
        &mut self,
        op: &impl LinearOperator<Host>,
        budget: usize,
        target: f64,
        refine: bool,
    ) -> Result<SolveReport<f64>, MnaError> {
        let policy =
            ConvergencePolicy::new(0., target, budget).map_err(|_| MnaError::InvalidOptions)?;
        let (b, x) = if refine {
            let (residual, correction) = self
                .refinement
                .get_or_insert_with(|| (vec![0.; self.x.len()], vec![0.; self.x.len()]));
            op.apply(&Host, &self.x, residual)
                .map_err(MnaError::Backend)?;
            for (r, &b) in residual.iter_mut().zip(&self.rhs) {
                *r = b - *r;
            }
            correction.fill(0.);
            (&*residual, correction)
        } else {
            (&self.rhs, &mut self.x)
        };
        let report = Gmres::<Host, RESTART>::solve_into(
            &Host,
            op,
            &Identity,
            b,
            x,
            &mut self.workspace,
            policy,
        )
        .map_err(|e| match e {
            athena_core::SolveError::Backend(reason) => MnaError::Backend(reason),
            _ => MnaError::Backend("inconsistent Krylov dimensions"),
        })?;
        if refine && report.converged() {
            let (_, correction) = self.refinement.as_ref().expect("correction allocated");
            for (v, &delta) in self.x.iter_mut().zip(correction) {
                *v += delta;
            }
        }
        Ok(report)
    }
}
