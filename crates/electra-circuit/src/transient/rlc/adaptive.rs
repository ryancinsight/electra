//! Step-doubling control over one constant-drive interval.
use super::{ElectricCurrent, ElectricPotential, RlcState, SeriesRlc, StepError, Time, step};

/// Maximum trial duration and positive absolute local error tolerances.
pub struct AdaptiveOptions {
    /// Largest full trial step.
    pub maximum_step: Time,
    /// Absolute capacitor voltage tolerance.
    pub voltage_error: ElectricPotential,
    /// Absolute inductor current tolerance.
    pub current_error: ElectricCurrent,
}

/// Endpoint of a constant-drive segment, with integration statistics.
pub struct SegmentResult {
    /// State at the specified endpoint.
    pub state: RlcState,
    /// Number of accepted pairs of half steps.
    pub accepted: usize,
    /// Number of discarded full/half-step comparisons.
    pub rejected: usize,
}

impl SeriesRlc {
    /// Integrate exactly to a prescribed source event or observation time. Call again
    /// with its new drive for the following segment. A rejected trial only changes
    /// the proposed step size. No trajectory storage is allocated.
    pub fn advance_segment(
        &self,
        initial: RlcState,
        drive: ElectricPotential,
        duration: Time,
        options: &AdaptiveOptions,
    ) -> Result<SegmentResult, StepError> {
        let end = step::interval(duration)?;
        let max = options.maximum_step.into_base();
        if !max.is_finite() || max <= 0. {
            return Err(StepError::Interval);
        }
        let vtol = options.voltage_error.into_base();
        let itol = options.current_error.into_base();
        if !vtol.is_finite() || vtol <= 0. || !itol.is_finite() || itol <= 0. {
            return Err(StepError::State);
        }
        let mut result = SegmentResult {
            state: self.advance(initial, drive, Time::default())?,
            accepted: 0,
            rejected: 0,
        };
        let mut elapsed = 0.;
        let mut proposed = max;
        while elapsed < end {
            let remaining = end - elapsed;
            let h = proposed.min(remaining);
            let half = Time::from_base(0.5 * h);
            if elapsed + h == elapsed || half.into_base() == 0. {
                return Err(StepError::Interval);
            }
            let coarse = self.advance(result.state, drive, Time::from_base(h))?;
            let middle = self.advance(result.state, drive, half)?;
            let fine = self.advance(middle, drive, half)?;
            // The midpoint method is second order. Dividing the difference by
            // 2² - 1 estimates the local error of the two-half-step solution.
            let ev = ((fine.voltage - coarse.voltage).into_base() / 3.).abs() / vtol;
            let ei = ((fine.current - coarse.current).into_base() / 3.).abs() / itol;
            let error = ev.max(ei);
            if error > 1. {
                result.rejected += 1;
                proposed = half.into_base();
            } else {
                result.state = fine;
                result.accepted += 1;
                elapsed = if h == remaining { end } else { elapsed + h };
                proposed = if error < 0.125 { (2. * h).min(max) } else { h };
            }
        }
        Ok(result)
    }
}
