//! Partial row pivoting on a breadth-first ordered complex band.
use super::{Complex64, ElectricPotential, Row};
use crate::ac::scaling::quotient;
mod storage;
use std::error::Error;
use storage::Matrix;

const MAX_HALF_BAND: usize = 32;

pub(super) fn solve(
    rows: &[Option<Row>],
    free: &[usize],
    volts: &mut [ElectricPotential<Complex64>],
    slots: &mut [usize],
    nodes: &mut Vec<usize>,
) -> Result<(), Box<dyn Error>> {
    let mut width = super::ordering::width(rows, slots, nodes);
    if width > MAX_HALF_BAND {
        // The last BFS node lies in the farthest level from the original seed.
        let seed = *nodes.last().expect("nonempty core");
        for &node in nodes.iter() {
            slots[node] = usize::MAX;
        }
        super::ordering::visit(rows, seed, slots, nodes);
        width = super::ordering::width(rows, slots, nodes);
    }
    if width > MAX_HALF_BAND {
        return Err(
            format!("irreducible AC core half-bandwidth {width} exceeds {MAX_HALF_BAND}").into(),
        );
    }
    let mut matrix = Matrix::new(nodes.len(), width);
    let mut rhs = Vec::with_capacity(nodes.len());
    for (i, &node) in nodes.iter().enumerate() {
        let row = rows[node].as_ref().unwrap();
        for (&j, value) in &row.edges {
            matrix[(i, slots[j])] = value.total();
        }
        matrix[(i, i)] = row.diagonal.total();
        rhs.push(row.rhs.total());
    }
    eliminate(&mut matrix, &mut rhs, width)?;
    for (&node, value) in nodes.iter().zip(rhs) {
        volts[free[node]] = ElectricPotential::from_base(value);
    }
    Ok(())
}

fn eliminate(
    matrix: &mut Matrix,
    rhs: &mut [Complex64],
    width: usize,
) -> Result<(), Box<dyn Error>> {
    for k in 0..rhs.len() {
        let end = (k + width + 1).min(rhs.len());
        let magnitude = |row: usize| {
            let z = matrix[(row, k)];
            z.re.abs().max(z.im.abs())
        };
        let pivot = (k..end)
            .max_by(|&i, &j| magnitude(i).total_cmp(&magnitude(j)))
            .unwrap();
        if magnitude(pivot) == 0. {
            return Err("singular AC band pivot".into());
        }
        let upper = (k + 2 * width + 1).min(rhs.len());
        matrix.swap_active(k, pivot, upper);
        rhs.swap(k, pivot);
        let pivot_rhs = rhs[k];
        for j in k + 1..end {
            let value = matrix[(j, k)];
            if value == Complex64::default() {
                continue;
            }
            let factor = quotient(value, matrix[(k, k)])?;
            for column in k + 1..upper {
                let coefficient = matrix[(k, column)];
                matrix[(j, column)] -= factor * coefficient;
            }
            rhs[j] -= factor * pivot_rhs;
        }
    }
    for k in (0..rhs.len()).rev() {
        let mut value = rhs[k];
        for j in k + 1..(k + 2 * width + 1).min(rhs.len()) {
            let entry = matrix[(k, j)];
            value -= entry * rhs[j];
        }
        rhs[k] = quotient(value, matrix[(k, k)])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successive_pivots_at_the_lower_band_edge_recover_complex_solution() {
        let n = 31;
        let width = 3;
        let mut matrix = Matrix::new(n, width);
        let expected: Vec<_> = (0..n)
            .map(|i| Complex64::new(i as f64 - 4., 2. - i as f64))
            .collect();
        let mut rhs = vec![Complex64::default(); n];
        for i in 0..n {
            for j in i.saturating_sub(width)..(i + width + 1).min(n) {
                let value = if i == j + width || (i < width && j == i + width) {
                    Complex64::new(3., 1.)
                } else if i == j {
                    Complex64::new(1., -0.5)
                } else {
                    Complex64::new(0.125, 0.)
                };
                matrix[(i, j)] = value;
                rhs[i] += value * expected[j];
            }
        }
        eliminate(&mut matrix, &mut rhs, width).unwrap();
        for (actual, reference) in rhs.iter().zip(expected) {
            assert!(
                (actual.re - reference.re).abs() < 1e-8 && (actual.im - reference.im).abs() < 1e-8
            );
        }
    }

    #[test]
    fn zero_diagonal_row_swap_preserves_second_superdiagonal_fill() {
        let mut matrix = Matrix::new(3, 1);
        for (i, j, value) in [
            (0, 1, 1.),
            (1, 0, 2.),
            (1, 1, 1.),
            (1, 2, 3.),
            (2, 1, 1.),
            (2, 2, 2.),
        ] {
            matrix[(i, j)] = Complex64::from(value);
        }
        let mut rhs = [2., 13., 8.].map(Complex64::from);
        eliminate(&mut matrix, &mut rhs, 1).unwrap();
        assert_eq!(rhs, [1., 2., 3.].map(Complex64::from));
    }
}
