//! A homogeneous right-hand side can satisfy every residual even at a singular feedback loop.
//! Prove numerical row independence once, before GMRES or the sweep predictor can accept zero.
mod assembly;
use super::System;
use crate::MnaError;
use assembly::SparseRows;

struct Pivot {
    start: usize,
    end: usize,
    scale: f64,
}
#[derive(Default)]
struct Row {
    entries: Vec<(usize, f64)>,
    // Bound the magnitude before cancellation, including row eliminations.
    scale: f64,
}
impl Row {
    fn add(&mut self, column: usize, value: f64) {
        if column == 0 || value == 0. {
            return;
        }
        match self.entries.binary_search_by_key(&column, |&(c, _)| c) {
            Ok(i) => {
                self.entries[i].1 += value;
                if self.entries[i].1 == 0. {
                    self.entries.remove(i);
                }
            }
            Err(i) => self.entries.insert(i, (column, value)),
        }
    }
    fn reduce(
        &mut self,
        equation: usize,
        previous: &[Pivot],
        coefficients: &[(usize, f64)],
        owner: &[usize],
    ) -> Result<usize, MnaError> {
        // Clear pivots in creation order. A stored row contains only later pivots or free columns.
        while let Some((i, prior)) = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, &(c, _))| (i, owner[c]))
            .filter(|&(_, prior)| prior != usize::MAX)
            .min_by_key(|&(_, prior)| prior)
        {
            let (_, factor) = self.entries.remove(i);
            let pivot = &previous[prior];
            self.scale += factor.abs() * pivot.scale;
            for &(c, k) in &coefficients[pivot.start..pivot.end] {
                self.add(c, -factor * k);
            }
        }
        let (index, &(_, weight)) = self
            .entries
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                // For equal magnitudes keep the natural diagonal. This preserves
                // sparsity and avoids growth from swapping a passive ladder's neighbors.
                a.1.abs()
                    .total_cmp(&b.1.abs())
                    .then_with(|| (a.0 == equation).cmp(&(b.0 == equation)))
            })
            .ok_or(MnaError::RankDeficient)?;
        if !weight.is_finite() || !self.scale.is_finite() {
            return Err(MnaError::Arithmetic);
        }
        if weight.abs() <= 64. * f64::EPSILON * self.scale {
            return Err(MnaError::RankDeficient);
        }
        let (column, _) = self.entries.remove(index);
        for (_, k) in &mut self.entries {
            *k /= weight;
        }
        self.scale /= weight.abs();
        Ok(column)
    }
}
pub(super) fn verify(system: &System) -> Result<(), MnaError> {
    let rows = SparseRows::new(system);
    let dimension = rows.offsets.len() - 1;
    let mut owner = vec![usize::MAX; dimension];
    let mut pivots = Vec::with_capacity(dimension - 1);
    let mut coefficients = Vec::new();
    let mut row = Row::default();
    for i in 1..dimension {
        row.entries.clear();
        row.scale = 0.;
        for &(column, value) in &rows.data[rows.offsets[i]..rows.offsets[i + 1]] {
            row.scale = row.scale.max(value.abs());
            row.add(column, value);
        }
        let column = row.reduce(i, &pivots, &coefficients, &owner)?;
        owner[column] = pivots.len();
        let start = coefficients.len();
        coefficients.extend_from_slice(&row.entries);
        pivots.push(Pivot {
            start,
            end: coefficients.len(),
            scale: row.scale,
        });
    }
    Ok(())
}
