//! In-place relaxation for borrowed adjacency lists and CSR storage.
use core::ops::{Add, Div, Mul};

/// One free-node equation. Fixed nodes are omitted from the sweep.
/// The neighbour iterator yields (node, positive conductance) in summation order.
pub struct RelaxationRow<T, N> {
    /// Dense index of the node to update.
    pub node: usize,
    /// Sum of all incident conductances, finite and positive.
    pub diagonal: T,
    /// Signed injection plus any eliminated fixed-neighbour terms.
    pub source: T,
    /// Neighbour indices and weights.
    pub neighbours: N,
}

/// Perform one Gauss–Seidel sweep in caller order, without allocations.
/// The caller owns anchoring, numerical validation and stopping policy.
/// # Panics
/// Panics if a row or neighbour refers outside the supplied potential slice.
pub fn sweep<T, N>(potentials: &mut [T], rows: impl IntoIterator<Item = RelaxationRow<T, N>>)
where
    T: Copy + Add<Output = T> + Mul<Output = T> + Div<Output = T>,
    N: IntoIterator<Item = (usize, T)>,
{
    for row in rows {
        let sum = row
            .neighbours
            .into_iter()
            .fold(row.source, |s, (j, g)| s + g * potentials[j]);
        potentials[row.node] = sum / row.diagonal;
    }
}
