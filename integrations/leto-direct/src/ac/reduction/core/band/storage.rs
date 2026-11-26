//! The lower band occupies w offsets and pivot fill can occupy 2w upper offsets.
use electra_ac::Complex64;
use std::ops::{Index, IndexMut};

pub(super) struct Matrix {
    entries: Vec<Complex64>,
    width: usize,
    stride: usize,
}

impl Matrix {
    pub(super) fn new(order: usize, width: usize) -> Self {
        let stride = 3 * width + 1;
        Self {
            entries: vec![Complex64::default(); order * stride],
            width,
            stride,
        }
    }

    fn offset(&self, row: usize, column: usize) -> usize {
        debug_assert!(column + self.width >= row && column <= row + 2 * self.width);
        row * self.stride + (column + self.width - row)
    }

    pub(super) fn swap_active(&mut self, k: usize, pivot: usize, end: usize) {
        if pivot != k {
            for j in k..end {
                let a = self.offset(k, j);
                let b = self.offset(pivot, j);
                self.entries.swap(a, b);
            }
        }
    }
}

impl Index<(usize, usize)> for Matrix {
    type Output = Complex64;
    fn index(&self, (row, col): (usize, usize)) -> &Complex64 {
        &self.entries[self.offset(row, col)]
    }
}

impl IndexMut<(usize, usize)> for Matrix {
    fn index_mut(&mut self, (row, col): (usize, usize)) -> &mut Complex64 {
        let offset = self.offset(row, col);
        &mut self.entries[offset]
    }
}
