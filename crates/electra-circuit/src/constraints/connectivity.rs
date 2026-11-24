fn root(links: &mut [usize], mut i: usize) -> usize {
    while links[i] != i {
        links[i] = links[links[i]];
        i = links[i];
    }
    i
}

pub(super) fn join(links: &mut [usize], a: usize, b: usize) -> bool {
    let a = root(links, a);
    let b = root(links, b);
    if a == b {
        return false;
    }
    links[b] = a;
    true
}

use crate::{Circuit, Element};

/// A passive/ideal-voltage component needs a route to reference for a unique potential.
pub(crate) fn floating_node(c: &Circuit) -> Option<usize> {
    let mut links: Vec<_> = (0..c.names.len()).collect();
    for part in &c.parts {
        if !matches!(part.element, Element::Current { .. }) {
            join(&mut links, part.positive, part.negative);
        }
    }
    let ground = root(&mut links, 0);
    (1..c.names.len()).find(|&n| root(&mut links, n) != ground)
}
