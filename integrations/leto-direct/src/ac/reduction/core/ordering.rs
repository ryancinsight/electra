//! Traverse one residual block and map its nodes to local factor indices.
use super::Row;

pub(super) fn visit(
    rows: &[Option<Row>],
    seed: usize,
    slots: &mut [usize],
    nodes: &mut Vec<usize>,
) {
    nodes.clear();
    nodes.push(seed);
    slots[seed] = 0;
    let mut cursor = 0;
    while cursor < nodes.len() {
        let node = nodes[cursor];
        for &next in rows[node].as_ref().unwrap().edges.keys() {
            if slots[next] == usize::MAX {
                slots[next] = nodes.len();
                nodes.push(next);
            }
        }
        cursor += 1;
    }
}

pub(super) fn width(rows: &[Option<Row>], slots: &[usize], nodes: &[usize]) -> usize {
    nodes
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
        .unwrap_or(0)
}
