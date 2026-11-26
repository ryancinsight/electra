//! Opt-in partial-pivoted Leto LU for bounded DC circuits.
//!
//! ```
//! use electra_circuit::parse;
//! use electra_leto_direct::PivotedLu;
//!
//! let circuit = parse("V1 in 0 9\nR1 in tap 1k\nR2 tap 0 2k").unwrap();
//! let point = circuit.operating_point_direct::<PivotedLu>(Default::default()).unwrap();
//! let tap = point.probes.volts.iter().find(|(name, _)| *name == "tap").unwrap().1;
//! assert!((tap.into_base() - 6.).abs() < 1e-12);
//! ```
#![forbid(unsafe_code)]
pub mod ac;
use electra_circuit::{
    MnaError,
    direct::{DirectFactor, Matrix},
};
use leto::{Array2, ArrayView1, ArrayViewMut1, Layout};
use leto_ops::application::linalg::lu::{LuDecomposition, lu_decompose};

/// Reusable dense LU factor. At the order limit two live square f64 matrices occupy 1 MiB.
pub struct PivotedLu(LuDecomposition<f64>);
impl DirectFactor for PivotedLu {
    const MAX_ORDER: usize = 256;

    fn factor(matrix: Matrix<'_>) -> Result<Self, MnaError> {
        let n = matrix.order();
        if n > Self::MAX_ORDER {
            return Err(MnaError::DirectOrder {
                order: n,
                limit: Self::MAX_ORDER,
            });
        }
        let mut values = vec![0.; n * n];
        matrix.for_each_entry(|row, column, value| values[row * n + column] += value);
        Self::from_values(n, values)
    }
    fn solve_into(&mut self, rhs: &[f64], solution: &mut [f64]) -> Result<(), MnaError> {
        self.solve(rhs, solution)
    }
}

impl PivotedLu {
    fn from_values(n: usize, values: Vec<f64>) -> Result<Self, MnaError> {
        let input = Array2::from_shape_vec([n, n], values).map_err(provider_error)?;
        lu_decompose(&input.view())
            .map(Self)
            .map_err(provider_error)
    }
    fn solve(&self, rhs: &[f64], solution: &mut [f64]) -> Result<(), MnaError> {
        let layout = Layout::c_contiguous([rhs.len()]).map_err(provider_error)?;
        let rhs = ArrayView1::try_new(layout, rhs).map_err(provider_error)?;
        let mut out = ArrayViewMut1::try_new(layout, solution).map_err(provider_error)?;
        self.0.solve_into(&rhs, &mut out).map_err(provider_error)
    }
}

fn provider_error(e: leto::LetoError) -> MnaError {
    MnaError::Direct(e.to_string())
}
