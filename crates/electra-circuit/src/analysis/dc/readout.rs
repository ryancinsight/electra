use super::{Excitation, Point};
use crate::{Circuit, DcResult, ElectricCurrent, ElectricPotential, Element, Part, SourceValue};

impl Part {
    pub(in crate::analysis) fn passive_current(
        &self,
        c: &Circuit,
        voltage_at: impl Fn(usize) -> ElectricPotential,
        index: usize,
        excitation: Excitation,
    ) -> Option<ElectricCurrent> {
        match self.element {
            Element::Resistor { branch } => Some(
                c.branches[branch].conductance
                    * (voltage_at(self.positive) - voltage_at(self.negative)),
            ),
            Element::Current { amperes } => Some(match excitation {
                Some((swept, SourceValue::Current(value))) if swept == index => value,
                _ => amperes,
            }),
            Element::Voltage { .. } => None,
        }
    }
}

impl DcResult<'_> {
    pub(super) fn grounded(&mut self, c: &Circuit, point: &Point, excitation: Excitation) {
        for ((_, out), &v) in self.volts.iter_mut().zip(&point.potentials) {
            *out = v;
        }
        for (index, ((_, out), part)) in self.amperes.iter_mut().zip(&c.parts).enumerate() {
            *out = part
                .passive_current(c, |node| point.potentials[node], index, None)
                .unwrap_or_else(|| {
                    if part.negative == 0 {
                        -point.reactions[part.positive]
                    } else {
                        point.reactions[part.negative]
                    }
                });
        }
        if let Some((index, SourceValue::Current(current))) = excitation {
            self.amperes[index].1 = current;
        }
        self.residual_amperes = point.max_residual;
        self.sweeps = point.sweeps;
    }
}
