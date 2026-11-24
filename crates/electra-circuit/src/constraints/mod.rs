//! Compile independent voltage constraints once, retaining original node and part identities.
mod cycles;
mod error;
pub(crate) use error::VoltageError;
mod forest;

use crate::{ElectricPotential, Element, Part, Resistor};
pub(crate) use forest::Forest;

#[derive(Debug)]
pub(crate) enum Constraints {
    Grounded(Vec<Option<ElectricPotential>>),
    Coupled(Forest),
}

impl Constraints {
    pub fn compile(
        nodes: usize,
        parts: &[Part],
        branches: &[Resistor],
    ) -> Result<Self, VoltageError> {
        let mut fixed = vec![None; nodes];
        fixed[0] = Some(ElectricPotential::default());
        for part in parts {
            if let Element::Voltage { volts, .. } = part.element {
                let node = part.positive.max(part.negative);
                if (part.positive != 0 && part.negative != 0)
                    || fixed[node].replace(part.step_to(node, volts)).is_some()
                {
                    drop(fixed);
                    return Forest::compile(nodes, parts, branches).map(Self::Coupled);
                }
            }
        }
        Ok(Self::Grounded(fixed))
    }
}

impl Part {
    /// Signed voltage increment on walking from the other terminal to this node.
    pub(crate) fn step_to(&self, node: usize, voltage: ElectricPotential) -> ElectricPotential {
        if node == self.positive {
            voltage
        } else {
            -voltage
        }
    }
}
