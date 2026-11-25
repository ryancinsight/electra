use crate::{AcError, Frequency};

/// Inclusive, allocation-free logarithmic frequency grid. Count includes both endpoints.
#[derive(Debug, Clone)]
pub struct LogSweep {
    start: Frequency,
    stop: Frequency,
    start_log: f64,
    span: f64,
    count: u32,
    index: u32,
    previous: Option<Frequency>,
}
impl LogSweep {
    /// Construct an ascending or descending grid. A single point has equal endpoints.
    pub fn new(start: Frequency, stop: Frequency, count: u32) -> Result<Self, AcError> {
        let a = start.into_base();
        let b = stop.into_base();
        if !a.is_finite()
            || !b.is_finite()
            || a <= 0.
            || b <= 0.
            || count == 0
            || (count == 1) != (a == b)
        {
            return Err(AcError::Grid);
        }
        let start_log = a.ln();
        let span = if (b - a).abs() <= a * 0.5 {
            ((b - a) / a).ln_1p()
        } else {
            b.ln() - start_log
        };
        Ok(Self {
            start,
            stop,
            start_log,
            span,
            count,
            index: 0,
            previous: None,
        })
    }
}
impl Iterator for LogSweep {
    type Item = Result<Frequency, AcError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.count {
            return None;
        }
        let index = self.index;
        self.index += 1;
        let f = if index == 0 {
            self.start
        } else if self.index == self.count {
            self.stop
        } else {
            let fraction = f64::from(index) / f64::from(self.count - 1);
            let step = self.span * fraction;
            let value = if step.abs() < 0.5 {
                self.start
                    .into_base()
                    .mul_add(step.exp_m1(), self.start.into_base())
            } else {
                (self.start_log + step).exp()
            };
            Frequency::from_base(value)
        };
        let direction = self.start.partial_cmp(&self.stop);
        if !f.as_base().is_finite()
            || self
                .previous
                .is_some_and(|p| p.partial_cmp(&f) != direction)
            || (self.index < self.count && f.partial_cmp(&self.stop) != direction)
        {
            self.index = self.count;
            return Some(Err(AcError::Grid));
        }
        self.previous = Some(f);
        Some(Ok(f))
    }
}
impl core::iter::FusedIterator for LogSweep {}
