pub(super) fn root(links: &mut [usize], mut i: usize) -> usize {
    while links[i] != i {
        links[i] = links[links[i]];
        i = links[i];
    }
    i
}

use crate::{Circuit, Element};

/// A passive/ideal-voltage component needs a route to reference for a unique potential.
pub(crate) fn floating_node(c: &Circuit) -> Option<usize> {
    let mut links: Vec<_> = (0..c.names.len()).collect();
    for part in &c.parts {
        if !matches!(part.element, Element::Current { .. }) {
            let a = root(&mut links, part.positive);
            let b = root(&mut links, part.negative);
            links[b] = a;
        }
    }
    let ground = root(&mut links, 0);
    (1..c.names.len()).find(|&n| root(&mut links, n) != ground)
}
