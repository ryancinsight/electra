//! Compare the provider dispatcher with an explicitly requested sparse numeric attempt.
use electra_circuit::{
    MnaError,
    direct::{DirectFactor, Matrix},
};
use leto::{ArrayView1, ArrayViewMut1, Layout, LetoError};
use leto_ops::application::sparse::{
    CooMatrix, CscMatrix, CsrMatrix, DENSE_LIMIT_DEFAULT, OwnedNumericLu, SparseLuSolver,
    factor_numeric, factor_symbolic,
};

fn csr(matrix: Matrix<'_>) -> CsrMatrix<f64> {
    let mut coo = CooMatrix::new(matrix.order(), matrix.order());
    matrix.for_each_entry(|r, c, v| coo.push(r, c, v));
    coo.to_csr()
}

pub struct Dispatch<const SPARSE_FIRST: bool>(OwnedNumericLu<f64>);
impl<const SPARSE_FIRST: bool> DirectFactor for Dispatch<SPARSE_FIRST> {
    const MAX_ORDER: usize = DENSE_LIMIT_DEFAULT;
    fn factor(matrix: Matrix<'_>) -> Result<Self, MnaError> {
        let csr = csr(matrix);
        let mut solver = SparseLuSolver::default();
        if SPARSE_FIRST {
            solver.small_switch = 0;
            solver.density_threshold = 2.;
        }
        let symbolic = factor_symbolic(&CscMatrix::from_csr(&csr.as_view()));
        solver
            .factor_sparse_with_symbolic(&csr, &symbolic)
            .map(Self)
            .map_err(|e| MnaError::Direct(e.to_string()))
    }
    fn solve_into(&mut self, rhs: &[f64], out: &mut [f64]) -> Result<(), MnaError> {
        fn views<'a>(
            rhs: &'a [f64],
            out: &'a mut [f64],
        ) -> leto::Result<(ArrayView1<'a, f64>, ArrayViewMut1<'a, f64>)> {
            let layout = Layout::c_contiguous([rhs.len()])?;
            Ok((
                ArrayView1::try_new(layout, rhs)?,
                ArrayViewMut1::try_new(layout, out)?,
            ))
        }
        views(rhs, out)
            .and_then(|(rhs, mut out)| self.0.solve_into(&rhs, &mut out))
            .map_err(|e| MnaError::Direct(e.to_string()))
    }
}

/// Diagnose the raw sparse attempt before returning an independently pivoted, checked solution.
pub struct Audit(electra_leto_direct::PivotedLu);
impl DirectFactor for Audit {
    const MAX_ORDER: usize = electra_leto_direct::PivotedLu::MAX_ORDER;
    fn factor(matrix: Matrix<'_>) -> Result<Self, MnaError> {
        let csr = csr(matrix);
        let csc = CscMatrix::from_csr(&csr.as_view());
        let symbolic = factor_symbolic(&csc);
        let defaults = SparseLuSolver::default();
        let status = match factor_numeric(&csc, &symbolic, defaults.pivot_tolerance) {
            Ok(_) => "sparse",
            Err(LetoError::NumericalBreakdown(_)) => "pivot-required",
            Err(LetoError::StorageError { reason }) if reason.contains("singular") => {
                "singular-pivot"
            }
            Err(e) => return Err(MnaError::Direct(e.to_string())),
        };
        eprintln!("order={} nnz={} raw={status}", matrix.order(), csr.nnz());
        electra_leto_direct::PivotedLu::factor(matrix).map(Self)
    }
    fn solve_into(&mut self, rhs: &[f64], out: &mut [f64]) -> Result<(), MnaError> {
        self.0.solve_into(rhs, out)
    }
}
