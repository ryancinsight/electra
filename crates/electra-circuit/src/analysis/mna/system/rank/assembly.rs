//! Two passes place all pre-elimination terms in one contiguous buffer.
use super::super::System;
use athena_core::LinearOperator;

pub(super) struct SparseRows {
    pub offsets: Vec<usize>,
    pub data: Vec<(usize, f64)>,
}
impl SparseRows {
    pub fn new(system: &System) -> Self {
        let dimension = system.dimension();
        let mut offsets = vec![0; dimension + 1];
        system.coefficients(|row: usize, _, _| offsets[row + 1] += 1);
        for i in 0..dimension {
            offsets[i + 1] += offsets[i];
        }
        let mut cursor = offsets[..dimension].to_vec();
        let mut data = vec![(0, 0.); offsets[dimension]];
        system.coefficients(|row: usize, column, value| {
            data[cursor[row]] = (column, value);
            cursor[row] += 1;
        });
        Self { offsets, data }
    }
}
