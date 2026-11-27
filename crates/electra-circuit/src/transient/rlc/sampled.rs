//! Sample-clock synchronization between a voltage drive and a one-bit ADC.
use super::{AdaptiveOptions, ElectricPotential, RlcCheckpoint, StepError};

/// Output transition at the end of a sample interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdcEdge {
    /// Sampled voltage reached or exceeded the threshold.
    Rising,
    /// Sampled voltage dropped below the threshold.
    Falling,
}

/// ADC transition and adaptive work for one completed clock interval.
pub struct ClockResult {
    /// Edge at the new sample index, if the digital output changed.
    pub edge: Option<AdcEdge>,
    /// Accepted pairs of half steps.
    pub accepted: usize,
    /// Rejected adaptive trials.
    pub rejected: usize,
}

impl RlcCheckpoint {
    /// Advance one sample with a held voltage and a fixed ADC threshold. The
    /// caller supplies the same threshold, drive schedule and adaptive controls
    /// after restart. Edges are observable only at clock boundaries. An error
    /// preserves the analog state and cursor so the interval can be retried.
    pub fn advance_clock(
        &mut self,
        drive: ElectricPotential,
        threshold: ElectricPotential,
        options: &AdaptiveOptions,
    ) -> Result<ClockResult, StepError> {
        let next = self.next_step.checked_add(1).ok_or(StepError::Interval)?;
        if !threshold.into_base().is_finite() {
            return Err(StepError::State);
        }
        if self.interval.into_base() <= 0. {
            return Err(StepError::Interval);
        }
        let result = self
            .model
            .advance_segment(self.state, drive, self.interval, options)?;
        let edge = match (
            self.state.voltage >= threshold,
            result.state.voltage >= threshold,
        ) {
            (false, true) => Some(AdcEdge::Rising),
            (true, false) => Some(AdcEdge::Falling),
            _ => None,
        };
        self.state = result.state;
        self.next_step = next;
        Ok(ClockResult {
            edge,
            accepted: result.accepted,
            rejected: result.rejected,
        })
    }
}
