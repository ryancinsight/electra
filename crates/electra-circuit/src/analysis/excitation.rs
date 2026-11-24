//! Physical source orientation and prescribed nodal injections shared by both DC solvers.
use crate::{Circuit, ElectricCurrent, ElectricPotential, Element, Part, SourceValue};

#[derive(Default, Clone, Copy)]
pub(super) struct CurrentSum {
    sum: ElectricCurrent,
    correction: ElectricCurrent,
}
impl CurrentSum {
    // Neumaier compensation preserves small currents between cancelling sources.
    pub fn add(&mut self, x: ElectricCurrent) {
        let next = self.sum + x;
        self.correction += if self.sum.abs() >= x.abs() {
            (self.sum - next) + x
        } else {
            (x - next) + self.sum
        };
        self.sum = next;
    }
    pub fn with(mut self, delta: ElectricCurrent) -> ElectricCurrent {
        self.add(delta);
        self.value()
    }
    pub fn value(self) -> ElectricCurrent {
        self.sum + self.correction
    }
}

pub(super) type Excitation = Option<(usize, SourceValue)>;

pub(super) fn voltage(part: &Part, index: usize, excitation: Excitation) -> ElectricPotential {
    if let Some((swept, SourceValue::Voltage(applied))) = excitation
        && swept == index
    {
        return applied;
    }
    let Element::Voltage { volts, .. } = part.element else {
        unreachable!()
    };
    volts
}

pub(super) fn injections(
    c: &Circuit,
    count: usize,
    swept: Option<usize>,
    group: impl Fn(usize) -> usize,
) -> Vec<CurrentSum> {
    let mut rhs = vec![CurrentSum::default(); count];
    for (i, p) in c.parts.iter().enumerate() {
        if Some(i) != swept
            && let Element::Current { amperes } = p.element
        {
            let (a, b) = (group(p.positive), group(p.negative));
            if a != b {
                rhs[a].add(-amperes);
                rhs[b].add(amperes);
            }
        }
    }
    rhs
}
