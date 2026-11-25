mod cursor;
mod predict;
use super::{MnaError, MnaOptions, MnaPoint, Session};
use crate::{Circuit, SourceValue, SweepError, SweepRange, analysis::sweep::SweepPlan};
pub(super) use cursor::Cursor;
use predict::Predictor;

/// One borrowed sample of a full MNA transfer sweep.
pub struct MnaSample<'p, 'c> {
    /// Excitation applied at this point.
    pub excitation: SourceValue,
    /// Checked physical results, valid until the next call.
    pub result: &'p MnaPoint<'c>,
}
/// Streaming MNA sweep sharing operator, Krylov workspace and result buffers across points.
pub struct MnaSweep<'a> {
    session: Session<'a>,
    cursor: Cursor,
    name: &'a str,
    predictor: Predictor,
}
impl Circuit {
    /// Prepare the deck's DC sweep using the full MNA operator.
    pub fn dc_sweep_mna(&self, limits: MnaOptions) -> Result<MnaSweep<'_>, MnaError> {
        MnaSweep::new(
            self,
            self.sweep
                .ok_or(MnaError::Sweep(SweepError::MissingDirective))?,
            limits,
        )
    }
    /// Sweep an independent source with a typed range using full MNA.
    pub fn sweep_source_mna(
        &self,
        name: &str,
        range: SweepRange,
        limits: MnaOptions,
    ) -> Result<MnaSweep<'_>, MnaError> {
        MnaSweep::new(
            self,
            self.plan(name, range).map_err(MnaError::Sweep)?,
            limits,
        )
    }
}
impl<'a> MnaSweep<'a> {
    fn new(c: &'a Circuit, plan: SweepPlan, limits: MnaOptions) -> Result<Self, MnaError> {
        let session = Session::new(c, limits, Some(plan.source))?;
        Ok(Self {
            predictor: Predictor::new(session.linear.x.len()),
            session,
            cursor: Cursor::new(plan),
            name: &c.parts[plan.source].name,
        })
    }
    /// Original source name.
    pub fn source_name(&self) -> &'a str {
        self.name
    }
    /// Advance one point. Exhaustion and errors terminate the stream.
    pub fn next_point(&mut self) -> Result<Option<MnaSample<'_, 'a>>, MnaError> {
        let Some(excitation) = self.cursor.advance(|source, value| {
            self.predictor.seed(value, &mut self.session.linear.x);
            self.session.run(Some((source, value)))
        })?
        else {
            return Ok(None);
        };
        Ok(Some(MnaSample {
            excitation,
            result: &self.session.point,
        }))
    }
}
