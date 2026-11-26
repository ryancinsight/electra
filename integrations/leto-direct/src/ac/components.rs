//! Visit each independent block of the frequency-specific reduced equations.
use electra_circuit::{AcDeck, AcSystem};
use std::error::Error;

pub(super) fn visit(
    deck: &AcDeck,
    system: &AcSystem,
    dense_limit: usize,
    mut solve: impl FnMut(&[usize], &[usize]) -> Result<(), Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let mut slots = vec![usize::MAX; deck.nodes().len()];
    let mut nodes = Vec::new();
    for (seed, fixed) in deck.boundary().iter().enumerate() {
        if fixed.is_some() || slots[seed] != usize::MAX {
            continue;
        }
        nodes.clear();
        nodes.push(seed);
        slots[seed] = 0;
        let mut next = 0;
        let mut anchored = false;
        while next < nodes.len() {
            let i = nodes[next];
            next += 1;
            anchored |= system.has_boundary_branch(i);
            for (j, _) in system.row(i) {
                if slots[j] == usize::MAX {
                    slots[j] = nodes.len();
                    nodes.push(j);
                }
            }
        }
        if !anchored {
            return Err(format!("unanchored AC component at {}", deck.nodes()[seed]).into());
        }
        // Preserve the deck's pivot ordering for the existing dense path.
        // Larger components retain parent-before-child traversal order.
        if nodes.len() <= dense_limit {
            nodes.sort_unstable();
            for (slot, &node) in nodes.iter().enumerate() {
                slots[node] = slot;
            }
        }
        solve(&nodes, &slots)?;
    }
    Ok(())
}
