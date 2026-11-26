//! Partial row pivoting on a breadth-first ordered complex band.
use super::{Complex64, ElectricPotential, Row};
use crate::ac::scaling::quotient;
use std::{collections::BTreeMap, error::Error};

const MAX_HALF_BAND: usize = 32;
type Entries = BTreeMap<usize, Complex64>;

pub(super) fn solve(
    rows: &[Option<Row>],
    free: &[usize],
    volts: &mut [ElectricPotential<Complex64>],
    slots: &[usize],
    nodes: &[usize],
) -> Result<(), Box<dyn Error>> {
    let width = nodes
        .iter()
        .enumerate()
        .flat_map(|(i, &node)| {
            rows[node]
                .as_ref()
                .unwrap()
                .edges
                .keys()
                .map(move |&j| i.abs_diff(slots[j]))
        })
        .max()
        .unwrap_or(0);
    if width > MAX_HALF_BAND {
        return Err(
            format!("irreducible AC core half-bandwidth {width} exceeds {MAX_HALF_BAND}").into(),
        );
    }
    let mut matrix = Vec::with_capacity(nodes.len());
    let mut rhs = Vec::with_capacity(nodes.len());
    for (i, &node) in nodes.iter().enumerate() {
        let row = rows[node].as_ref().unwrap();
        let mut entries: Entries = row
            .edges
            .iter()
            .map(|(&j, value)| (slots[j], value.total()))
            .collect();
        entries.insert(i, row.diagonal.total());
        matrix.push(entries);
        rhs.push(row.rhs.total());
    }
    eliminate(&mut matrix, &mut rhs, width)?;
    for (&node, value) in nodes.iter().zip(rhs) {
        volts[free[node]] = ElectricPotential::from_base(value);
    }
    Ok(())
}

fn eliminate(
    matrix: &mut [Entries],
    rhs: &mut [Complex64],
    width: usize,
) -> Result<(), Box<dyn Error>> {
    for k in 0..matrix.len() {
        let end = (k + width + 1).min(matrix.len());
        let magnitude = |row: usize| {
            matrix[row]
                .get(&k)
                .map_or(0., |z| z.re.abs().max(z.im.abs()))
        };
        let pivot = (k..end)
            .max_by(|&i, &j| magnitude(i).total_cmp(&magnitude(j)))
            .unwrap();
        if magnitude(pivot) == 0. {
            return Err("singular AC band pivot".into());
        }
        matrix.swap(k, pivot);
        rhs.swap(k, pivot);
        let (done, active) = matrix[..end].split_at_mut(k + 1);
        let pivot_row = &done[k];
        let pivot_rhs = rhs[k];
        for (i, row) in active.iter_mut().enumerate() {
            let Some(value) = row.remove(&k) else {
                continue;
            };
            if value == Complex64::default() {
                continue;
            }
            let factor = quotient(value, pivot_row[&k])?;
            for (&column, &coefficient) in pivot_row.range(k + 1..) {
                let entry = row.entry(column).or_default();
                *entry -= factor * coefficient;
            }
            let j = k + i + 1;
            rhs[j] -= factor * pivot_rhs;
        }
    }
    for k in (0..matrix.len()).rev() {
        let mut value = rhs[k];
        for (&j, &entry) in matrix[k].range(k + 1..) {
            value -= entry * rhs[j];
        }
        rhs[k] = quotient(value, matrix[k][&k])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_diagonal_row_swap_preserves_second_superdiagonal_fill() {
        let row = |values: &[(usize, f64)]| {
            values
                .iter()
                .map(|&(i, v)| (i, Complex64::from(v)))
                .collect()
        };
        let mut matrix = [
            row(&[(1, 1.)]),
            row(&[(0, 2.), (1, 1.), (2, 3.)]),
            row(&[(1, 1.), (2, 2.)]),
        ];
        let mut rhs = [2., 13., 8.].map(Complex64::from);
        eliminate(&mut matrix, &mut rhs, 1).unwrap();
        assert_eq!(rhs, [1., 2., 3.].map(Complex64::from));
    }
}
