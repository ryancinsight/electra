use super::{DirectFactor, Session};
use crate::analysis::{mna::sweep::Cursor, sweep::SweepPlan};
use crate::{Circuit, MnaError, MnaOptions, MnaSample, SweepError, SweepRange};

/// Streaming DC transfer curve. One numeric factor serves every excitation in the sweep.
pub struct DirectSweep<'a, F> {
    session: Session<'a, F>,
    cursor: Cursor,
    name: &'a str,
}
impl Circuit {
    /// Use a direct factor for the sweep declared in the netlist.
    pub fn dc_sweep_direct<F: DirectFactor>(
        &self,
        limits: MnaOptions,
    ) -> Result<DirectSweep<'_, F>, MnaError> {
        DirectSweep::new(
            self,
            self.sweep
                .ok_or(MnaError::Sweep(SweepError::MissingDirective))?,
            limits,
        )
    }
    /// Drive a named independent source over a physically typed range with a direct factor.
    pub fn sweep_source_direct<F: DirectFactor>(
        &self,
        name: &str,
        range: SweepRange,
        limits: MnaOptions,
    ) -> Result<DirectSweep<'_, F>, MnaError> {
        DirectSweep::new(
            self,
            self.plan(name, range).map_err(MnaError::Sweep)?,
            limits,
        )
    }
}
impl<'a, F: DirectFactor> DirectSweep<'a, F> {
    fn new(c: &'a Circuit, plan: SweepPlan, limits: MnaOptions) -> Result<Self, MnaError> {
        Ok(Self {
            session: Session::new(c, limits, Some(plan.source))?,
            name: &c.parts[plan.source].name,
            cursor: Cursor::new(plan),
        })
    }
    /// Original name of the source being swept.
    pub fn source_name(&self) -> &'a str {
        self.name
    }
    /// Solve and check one point, borrowing the readout until the next advance.
    pub fn next_point(&mut self) -> Result<Option<MnaSample<'_, 'a>>, MnaError> {
        Ok(self
            .cursor
            .advance(|source, value| self.session.run(Some((source, value))))?
            .map(|excitation| MnaSample {
                excitation,
                result: &self.session.point,
            }))
    }
}
