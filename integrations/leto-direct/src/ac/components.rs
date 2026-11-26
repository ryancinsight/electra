//! Visit each independent block of the frequency-specific reduced equations.
use electra_circuit::{AcDeck, AcSystem};
use std::error::Error;

pub(super) fn visit(
    deck: &AcDeck,
    system: &AcSystem,
    limit: usize,
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
                    if nodes.len() == limit {
                        return Err(format!(
                            "AC component containing {} exceeds {limit} free nodes",
                            deck.nodes()[seed]
                        )
                        .into());
                    }
                    slots[j] = nodes.len();
                    nodes.push(j);
                }
            }
        }
        if !anchored {
            return Err(format!("unanchored AC component at {}", deck.nodes()[seed]).into());
        }
        // Keep pivot ordering independent of traversal order and consistent with deck indices.
        nodes.sort_unstable();
        for (slot, &node) in nodes.iter().enumerate() {
            slots[node] = slot;
        }
        solve(&nodes, &slots)?;
    }
    Ok(())
}
