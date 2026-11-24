use super::SweepError;
use aequitas::Quantity;

/// Linear sweep in the source's SI dimension. The sign of the step selects
/// direction. The endpoint is included if it lies on the grid.
#[derive(Debug, Clone, Copy)]
pub struct LinearSweep<D> {
    /// First excitation.
    pub start: Quantity<f64, D>,
    /// Inclusive limit.
    pub stop: Quantity<f64, D>,
    /// Nonzero signed spacing.
    pub step: Quantity<f64, D>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Grid<D> {
    range: LinearSweep<D>,
    index: f64,
    last: f64,
    lands_on_stop: bool,
    previous: Option<Quantity<f64, D>>,
}

impl<D> Grid<D> {
    pub fn new(range: LinearSweep<D>) -> Result<Self, SweepError> {
        let (start, stop, step) = (
            range.start.into_base(),
            range.stop.into_base(),
            range.step.into_base(),
        );
        if ![start, stop, step].into_iter().all(f64::is_finite) || step == 0.0 {
            return Err(SweepError::InvalidRange(
                "sweep needs finite bounds and a finite nonzero step",
            ));
        }
        if (stop > start && step < 0.0) || (stop < start && step > 0.0) {
            return Err(SweepError::InvalidRange(
                "sweep step points away from the stop",
            ));
        }
        let span = stop - start;
        let intervals = if span.is_finite() {
            span / step
        } else {
            stop / step - start / step
        };
        // Beyond this, consecutive integer sample indices cannot be represented.
        if intervals >= 2.0_f64.powi(53) {
            return Err(SweepError::InvalidRange(
                "sweep grid exceeds exact floating-point index range",
            ));
        }
        let nearest = intervals.round();
        let lands_on_stop = start == stop
            || (nearest >= 1.0
                && (nearest - intervals).abs() <= (4.0 * f64::EPSILON * intervals).min(0.25));
        let last = if lands_on_stop {
            nearest
        } else {
            intervals.floor()
        };
        Ok(Self {
            range,
            index: 0.0,
            last,
            lands_on_stop,
            previous: None,
        })
    }

    pub fn next_value(&mut self) -> Result<Option<Quantity<f64, D>>, SweepError> {
        if self.index > self.last {
            return Ok(None);
        }
        let value = if self.index == 0.0 {
            self.range.start
        } else if self.index == self.last && self.lands_on_stop {
            self.range.stop
        } else {
            // Keep the source dimension at the boundary of f64's fused operation.
            Quantity::from_base(
                self.range
                    .step
                    .as_base()
                    .mul_add(self.index, self.range.start.into_base()),
            )
        };
        let ascending = self.range.step.into_base() > 0.0;
        if !value.as_base().is_finite()
            || self.previous.is_some_and(|p| {
                if ascending {
                    value <= p || value > self.range.stop
                } else {
                    value >= p || value < self.range.stop
                }
            })
        {
            return Err(SweepError::InvalidRange(
                "successive sweep values are not representable",
            ));
        }
        self.index += 1.0;
        self.previous = Some(value);
        Ok(Some(value))
    }
}
