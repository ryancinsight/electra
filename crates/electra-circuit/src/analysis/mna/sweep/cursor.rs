use crate::analysis::sweep::SweepPlan;
use crate::{MnaError, SourceValue};

/// Shared end-of-stream and excitation-specific failure semantics for both MNA algorithms.
pub(in crate::analysis::mna) struct Cursor {
    plan: SweepPlan,
    finished: bool,
}
impl Cursor {
    pub fn new(plan: SweepPlan) -> Self {
        Self {
            plan,
            finished: false,
        }
    }
    pub fn advance(
        &mut self,
        solve: impl FnOnce(usize, SourceValue) -> Result<(), MnaError>,
    ) -> Result<Option<SourceValue>, MnaError> {
        if self.finished {
            return Ok(None);
        }
        self.finished = true;
        let Some(value) = self.plan.next_value().map_err(MnaError::Sweep)? else {
            return Ok(None);
        };
        solve(self.plan.source, value).map_err(|e| MnaError::Point(value, Box::new(e)))?;
        self.finished = false;
        Ok(Some(value))
    }
}
