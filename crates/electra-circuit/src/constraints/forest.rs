use super::{VoltageError, cycles};
use crate::{Element, Part, Resistor};
use electra_network::Branch;

#[derive(Debug)]
pub(crate) struct TreeEdge {
    pub parent: usize,
    pub child: usize,
    pub source: usize,
}

#[derive(Debug)]
pub(crate) struct Forest {
    pub group: Vec<usize>,
    pub roots: Vec<usize>,
    /// Parent-before-child traversal. The reverse order gives source reaction currents.
    pub tree: Vec<TreeEdge>,
    pub branches: Vec<Resistor>,
}

fn root(links: &mut [usize], mut i: usize) -> usize {
    while links[i] != i {
        links[i] = links[links[i]];
        i = links[i];
    }
    i
}

impl Forest {
    pub fn compile(n: usize, parts: &[Part], resistors: &[Resistor]) -> Result<Self, VoltageError> {
        let mut links: Vec<_> = (0..n).collect();
        let mut degree = vec![0; n + 1];
        let mut sources = Vec::new();
        let mut chords = Vec::new();
        // Select tree edges in deck order so that diagnostics identify the card
        // which closes a cycle even if BFS would visit its endpoints earlier.
        for (index, p) in parts.iter().enumerate() {
            if matches!(p.element, Element::Voltage { .. }) {
                let a = root(&mut links, p.positive);
                let b = root(&mut links, p.negative);
                if a == b {
                    chords.push(index);
                } else {
                    links[b] = a;
                    sources.push(index);
                    degree[p.positive + 1] += 1;
                    degree[p.negative + 1] += 1;
                }
            }
        }
        for node in 0..n {
            degree[node + 1] += degree[node];
        }
        let mut adjacency = vec![0; degree[n]];
        links.copy_from_slice(&degree[..n]);
        for &source in &sources {
            let p = &parts[source];
            for node in [p.positive, p.negative] {
                adjacency[links[node]] = source;
                links[node] += 1;
            }
        }
        let mut group = vec![usize::MAX; n];
        let mut roots = Vec::with_capacity(n - sources.len());
        let mut tree = Vec::with_capacity(sources.len());
        let mut stack = sources;
        stack.clear();
        for node in 0..n {
            if group[node] != usize::MAX {
                continue;
            }
            group[node] = roots.len();
            roots.push(node);
            stack.push(node);
            while let Some(parent) = stack.pop() {
                for &source in &adjacency[degree[parent]..degree[parent + 1]] {
                    let part = &parts[source];
                    let child = if parent == part.positive {
                        part.negative
                    } else {
                        part.positive
                    };
                    if group[child] == usize::MAX {
                        group[child] = group[parent];
                        stack.push(child);
                        tree.push(TreeEdge {
                            parent,
                            child,
                            source,
                        });
                    }
                }
            }
        }
        drop((degree, links, adjacency, stack));
        if !chords.is_empty() {
            return Err(cycles::diagnose(n, parts, &tree, &chords));
        }
        let branches = resistors
            .iter()
            .filter_map(|edge| {
                let (from, to) = (group[edge.from], group[edge.to]);
                (from != to).then_some(Branch {
                    from,
                    to,
                    conductance: edge.conductance,
                })
            })
            .collect();
        Ok(Self {
            group,
            roots,
            tree,
            branches,
        })
    }
}
