use super::{Error, Si};
use crate::{Branch, RelaxationRow};
use std::ops::AddAssign;

/// Free–free conductances in input order, plus the free–fixed RHS terms.
pub(super) struct Topology<'a, G> {
    pub branches: &'a [Branch<G>],
    pub fixed: Vec<bool>,
    rows: Vec<FreeRow<G>>,
    neighbours: Vec<(usize, Si<G>)>,
    pub boundary: Vec<(usize, usize, Si<G>)>,
}

struct FreeRow<G> {
    node: usize,
    diagonal: Si<G>,
    neighbours: usize,
}

impl<'a, G> Topology<'a, G>
where
    Si<G>: AddAssign,
{
    pub fn new<P, F>(fixed: &[Option<Si<P>>], branches: &'a [Branch<G>]) -> Result<Self, Error<F>> {
        let n = fixed.len();
        if n == 0 || fixed.iter().flatten().any(|q| !q.as_base().is_finite()) {
            return Err(Error::InvalidInput);
        }
        let fixed: Vec<_> = fixed.iter().map(Option::is_some).collect();
        let mut offsets = vec![0; n + 1];
        let mut diagonal = vec![Si::default(); n];
        let mut boundary_edges = 0;
        for (index, edge) in branches.iter().enumerate() {
            if edge.from >= n
                || edge.to >= n
                || edge.from == edge.to
                || !edge.conductance.as_base().is_finite()
                || edge.conductance <= Si::default()
            {
                return Err(Error::InvalidBranch(index));
            }
            for (row, other) in edge.free_ends([fixed[edge.from], fixed[edge.to]]) {
                diagonal[row] += edge.conductance;
                if fixed[other] {
                    boundary_edges += 1;
                } else {
                    offsets[row + 1] += 1;
                }
            }
        }
        for i in 0..n {
            offsets[i + 1] += offsets[i];
        }
        let mut neighbours = vec![(0, Si::default()); offsets[n]];
        let mut cursor = offsets[..n].to_vec();
        let mut boundary = Vec::with_capacity(boundary_edges);
        for edge in branches {
            for (row, other) in edge.free_ends([fixed[edge.from], fixed[edge.to]]) {
                if fixed[other] {
                    boundary.push((row, other, edge.conductance));
                } else {
                    neighbours[cursor[row]] = (other, edge.conductance);
                    cursor[row] += 1;
                }
            }
        }
        check_anchors(&fixed, &boundary, &offsets, &neighbours, cursor)?;
        if diagonal.iter().any(|q| !q.as_base().is_finite()) {
            return Err(Error::Arithmetic);
        }
        let mut rows = Vec::with_capacity(fixed.iter().filter(|&&v| !v).count());
        for (node, diagonal) in diagonal.into_iter().enumerate() {
            if !fixed[node] {
                rows.push(FreeRow {
                    node,
                    diagonal,
                    neighbours: offsets[node + 1] - offsets[node],
                });
            }
        }
        Ok(Self {
            branches,
            fixed,
            rows,
            neighbours,
            boundary,
        })
    }

    pub fn equations<'s, F>(
        &'s self,
        rhs: &'s [Si<F>],
    ) -> impl Iterator<Item = RelaxationRow<G, F, impl Iterator<Item = (usize, Si<G>)> + 's>> {
        let mut remaining = self.neighbours.as_slice();
        self.rows.iter().map(move |row| {
            let (run, tail) = remaining.split_at(row.neighbours);
            remaining = tail;
            RelaxationRow {
                node: row.node,
                source: rhs[row.node],
                diagonal: row.diagonal,
                neighbours: run.iter().copied(),
            }
        })
    }
}

fn check_anchors<G, F>(
    fixed: &[bool],
    boundary: &[(usize, usize, Si<G>)],
    offsets: &[usize],
    neighbours: &[(usize, Si<G>)],
    mut stack: Vec<usize>,
) -> Result<(), Error<F>> {
    let mut seen = fixed.to_vec();
    stack.clear();
    for &(row, _, _) in boundary {
        if !seen[row] {
            seen[row] = true;
            stack.push(row);
        }
    }
    while let Some(i) = stack.pop() {
        for &(j, _) in &neighbours[offsets[i]..offsets[i + 1]] {
            if !seen[j] {
                seen[j] = true;
                stack.push(j);
            }
        }
    }
    match seen.iter().position(|&found| !found) {
        Some(node) => Err(Error::Floating(node)),
        None => Ok(()),
    }
}
