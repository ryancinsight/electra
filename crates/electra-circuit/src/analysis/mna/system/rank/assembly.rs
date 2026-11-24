//! Two passes place all pre-elimination terms in one contiguous buffer.
use super::super::{Signal, Stamp, System, operator::Equations};
use athena_core::LinearOperator;

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
pub(super) struct SparseRows {
    pub offsets: Vec<usize>,
    pub data: Vec<(usize, f64)>,
}
impl SparseRows {
    pub fn new(system: &System) -> Self {
        let dimension = system.dimension();
        let mut offsets = vec![0; dimension + 1];
        system.stamp(&mut Coefficients(|row: usize, _, _| offsets[row + 1] += 1));
        for i in 0..dimension {
            offsets[i + 1] += offsets[i];
        }
        let mut cursor = offsets[..dimension].to_vec();
        let mut data = vec![(0, 0.); offsets[dimension]];
        system.stamp(&mut Coefficients(|row: usize, column, value| {
            data[cursor[row]] = (column, value);
            cursor[row] += 1;
        }));
        Self { offsets, data }
    }
}
