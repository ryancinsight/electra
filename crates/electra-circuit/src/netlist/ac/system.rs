//! Sparse original-node equations shared by export and numerical backends.
use super::{AcDeck, ParseError};
use electra_ac::{Complex64, ElectricConductance, ElectricCurrent, Frequency, PhasorSum};
use std::collections::BTreeMap;

/// Compensated coefficient and current sums of the reduced phasor equations.
pub struct AcSystem {
    matrix: BTreeMap<(usize, usize), PhasorSum>,
    rhs: Vec<PhasorSum>,
    anchored: Vec<bool>,
}

impl AcDeck {
    /// Assemble Yv = b in siemens and amperes at one frequency.
    pub fn assemble(&self, frequency: Frequency) -> Result<AcSystem, ParseError> {
        let mut matrix = BTreeMap::<_, PhasorSum>::new();
        let mut rhs = vec![PhasorSum::default(); self.nodes().len()];
        let mut boundary_y = vec![PhasorSum::default(); rhs.len()];
        for &(line, branch) in &self.branches {
            let [a, b] = branch.endpoints();
            let one_fixed = self.boundary[a].is_some() != self.boundary[b].is_some();
            let mut matrix_ok = true;
            let mut rhs_ok = true;
            branch
                .stamp(
                    frequency,
                    &self.boundary,
                    |i, j, y| {
                        if one_fixed {
                            matrix_ok &= add(&mut boundary_y[i], y.into_base());
                        }
                        matrix_ok &= add(matrix.entry((i, j)).or_default(), y.into_base());
                    },
                    |i, b| {
                        rhs_ok &= add(&mut rhs[i], b.into_base());
                    },
                )
                .map_err(|e| ParseError::new(line, e.to_string()))?;
            if !matrix_ok || !rhs_ok {
                return Err(ParseError::new(
                    line,
                    "AC coefficient accumulation overflow",
                ));
            }
        }
        Ok(AcSystem {
            matrix,
            rhs,
            anchored: boundary_y
                .into_iter()
                .map(|sum| sum.total() != Complex64::default())
                .collect(),
        })
    }
}
impl AcSystem {
    /// Nonzero admittance entries with original node indices, in row-major order.
    pub fn coefficients(
        &self,
    ) -> impl Iterator<Item = (usize, usize, ElectricConductance<Complex64>)> + '_ {
        self.matrix.iter().filter_map(entry)
    }
    /// Nonzero entries of one original node row in column order.
    pub fn row(
        &self,
        node: usize,
    ) -> impl Iterator<Item = (usize, ElectricConductance<Complex64>)> + '_ {
        self.matrix
            .range((node, 0)..=(node, usize::MAX))
            .filter_map(entry)
            .map(|(_, j, y)| (j, y))
    }
    /// Injected currents in original node order, including zeros.
    pub fn currents(&self) -> impl Iterator<Item = ElectricCurrent<Complex64>> + '_ {
        self.rhs
            .iter()
            .map(|sum| ElectricCurrent::from_base(sum.total()))
    }
    /// Whether the net admittance from this row to prescribed nodes is nonzero.
    pub fn has_boundary_branch(&self, node: usize) -> bool {
        self.anchored.get(node).copied().unwrap_or(false)
    }
}

fn add(sum: &mut PhasorSum, z: Complex64) -> bool {
    sum.add(z);
    let total = sum.total();
    total.re.is_finite() && total.im.is_finite()
}

fn entry(
    (&(i, j), value): (&(usize, usize), &PhasorSum),
) -> Option<(usize, usize, ElectricConductance<Complex64>)> {
    let y = value.total();
    (y != Complex64::default()).then(|| (i, j, ElectricConductance::from_base(y)))
}
