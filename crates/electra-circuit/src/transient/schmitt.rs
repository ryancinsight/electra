//! Sampled Schmitt receiver with inertial propagation delay.
use super::{ElectricPotential, StepError, Time, step};

/// A delivered logic level and its scheduled time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogicTransition {
    /// Absolute time of delivery.
    pub time: Time,
    /// Logic level following delivery.
    pub high: bool,
}

/// One-bit receiver with inclusive switching thresholds. Between thresholds the
/// input latch retains its last level. Only one delayed transition is stored.
pub struct SchmittReceiver {
    low: ElectricPotential,
    high: ElectricPotential,
    delay: Time,
    time: Time,
    latched: bool,
    output: bool,
    pending: Option<LogicTransition>,
}

impl SchmittReceiver {
    /// Initialize input and output to the same level at time zero.
    pub fn new(
        low: ElectricPotential,
        high: ElectricPotential,
        delay: Time,
        initial: bool,
    ) -> Result<Self, StepError> {
        step::interval(delay)?;
        if !low.into_base().is_finite() || !high.into_base().is_finite() || low >= high {
            return Err(StepError::State);
        }
        Ok(Self {
            low,
            high,
            delay,
            time: Time::default(),
            latched: initial,
            output: initial,
            pending: None,
        })
    }

    /// Next output deadline, for callers that schedule explicit wakeups.
    pub fn deadline(&self) -> Option<Time> {
        self.pending.map(|event| event.time)
    }

    /// Deliver any due event before sampling the input. Equal-time samples use
    /// caller order. Reversals before the deadline cancel a pending pulse.
    /// A late poll returns the original delivery timestamp.
    pub fn observe(
        &mut self,
        time: Time,
        voltage: ElectricPotential,
    ) -> Result<Option<LogicTransition>, StepError> {
        step::interval(time)?;
        if time < self.time {
            return Err(StepError::Interval);
        }
        if !voltage.into_base().is_finite() {
            return Err(StepError::State);
        }
        let latch = if voltage >= self.high {
            true
        } else if voltage <= self.low {
            false
        } else {
            self.latched
        };
        let mut delivered = self.pending.filter(|event| event.time <= time);
        let output = delivered.map_or(self.output, |event| event.high);
        let mut pending = self.pending.filter(|event| event.time > time);
        if latch != self.latched {
            pending = None;
            if latch != output {
                let due = time + self.delay;
                if !due.into_base().is_finite() || (self.delay.into_base() > 0. && due == time) {
                    return Err(StepError::Interval);
                }
                let event = LogicTransition {
                    time: due,
                    high: latch,
                };
                if due == time {
                    delivered = Some(event);
                } else {
                    pending = Some(event);
                }
            }
        }
        self.time = time;
        self.latched = latch;
        self.pending = pending;
        self.output = delivered.map_or(output, |event| event.high);
        Ok(delivered)
    }
}
