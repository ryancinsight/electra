//! Scalar entries for consumers that need the individual matrix coefficients.
use super::{Signal, Stamp, System, operator::Equations};

struct Coefficients<F>(F);
impl<F: FnMut(usize, usize, f64)> Equations for Coefficients<F> {
    fn inject(&mut self, stamp: &Stamp, signal: Signal) {
        self.voltage(stamp.a, signal, stamp.at_a);
        self.voltage(stamp.b, signal, -stamp.at_b);
    }
    fn voltage(&mut self, row: usize, signal: Signal, weight: f64) {
        if row != 0 && weight != 0. {
            signal.coefficients(|column, sign| {
                if column != 0 {
                    (self.0)(row, column, weight * sign);
                }
            });
        }
    }
}
impl System {
    pub(in crate::analysis::mna) fn coefficients(&self, visit: impl FnMut(usize, usize, f64)) {
        self.stamp(&mut Coefficients(visit));
    }
}
