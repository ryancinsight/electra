//! In-place relaxation for borrowed adjacency lists and CSR storage.
use aequitas::Quantity;
use core::ops::{Add, Div, Mul};

/// One free-node equation. Fixed nodes are omitted from the sweep.
/// The neighbour iterator yields (node, positive conductance) in summation order.
pub struct RelaxationRow<G, F, N, T = f64> {
    /// Dense index of the node to update.
    pub node: usize,
    /// Sum of all incident conductances, finite and positive.
    pub diagonal: Quantity<T, G>,
    /// Signed injection plus any eliminated fixed-neighbour terms.
    pub source: Quantity<T, F>,
    /// Neighbour indices and weights.
    pub neighbours: N,
}

/// Perform one Gauss–Seidel sweep in caller order, without allocations.
/// The caller owns anchoring, numerical validation and stopping policy.
/// # Panics
/// Panics if a row or neighbour refers outside the supplied potential slice.
pub fn sweep<T: Copy, P, G, F, N>(
    potentials: &mut [Quantity<T, P>],
    rows: impl IntoIterator<Item = RelaxationRow<G, F, N, T>>,
) where
    Quantity<T, G>: Mul<Quantity<T, P>, Output = Quantity<T, F>>,
    Quantity<T, F>: Add<Output = Quantity<T, F>> + Div<Quantity<T, G>, Output = Quantity<T, P>>,
    N: IntoIterator<Item = (usize, Quantity<T, G>)>,
{
    for row in rows {
        let sum = row
            .neighbours
            .into_iter()
            .fold(row.source, |s, (j, g)| s + g * potentials[j]);
        potentials[row.node] = sum / row.diagonal;
    }
}
