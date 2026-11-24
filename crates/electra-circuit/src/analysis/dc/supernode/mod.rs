mod projection;

use super::{Excitation, voltage};
use crate::analysis::excitation::CurrentSum;
use crate::{
    Circuit, ElectricCurrent, ElectricPotential, Element, SolveError, constraints::Forest,
};

/// Root potential plus a tree offset recovers every original node voltage.
pub(super) struct Supernodes<'a> {
    pub forest: &'a Forest,
    offsets: Vec<ElectricPotential>,
    /// Constant independent-current terms retained only when varying a voltage.
    voltage_sweep_bias: Vec<ElectricCurrent>,
    imbalance: Vec<CurrentSum>,
    scale: Vec<ElectricCurrent>,
}

impl<'a> Supernodes<'a> {
    pub fn prepare(
        c: &Circuit,
        forest: &'a Forest,
        swept: Option<usize>,
        mut rhs: Vec<CurrentSum>,
    ) -> Result<(Self, Vec<CurrentSum>), SolveError> {
        let varying_voltage =
            swept.is_some_and(|i| matches!(c.parts[i].element, Element::Voltage { .. }));
        let n = c.names.len();
        let mut state = Self {
            forest,
            offsets: vec![ElectricPotential::default(); n],
            voltage_sweep_bias: if varying_voltage
                && rhs.iter().any(|q| q.value() != ElectricCurrent::default())
            {
                rhs.iter().copied().map(CurrentSum::value).collect()
            } else {
                Vec::new()
            },
            imbalance: vec![CurrentSum::default(); n],
            scale: vec![ElectricCurrent::default(); forest.roots.len()],
        };
        if !varying_voltage {
            state.stamp_offsets(c, None, |n, amps| rhs[n].add(amps))?;
            if rhs.iter().any(|i| !i.value().as_base().is_finite()) {
                return Err(SolveError::Arithmetic);
            }
        }
        Ok((state, rhs))
    }

    pub fn set_voltage(
        &mut self,
        c: &Circuit,
        excitation: Excitation,
        rhs: &mut [ElectricCurrent],
    ) -> Result<(), SolveError> {
        if self.voltage_sweep_bias.is_empty() {
            rhs.fill(ElectricCurrent::default());
        } else {
            rhs.copy_from_slice(&self.voltage_sweep_bias);
        }
        self.stamp_offsets(c, excitation, |n, i| rhs[n] += i)?;
        if rhs.iter().any(|i| !i.as_base().is_finite()) {
            return Err(SolveError::Arithmetic);
        }
        Ok(())
    }

    fn stamp_offsets(
        &mut self,
        c: &Circuit,
        excitation: Excitation,
        mut inject: impl FnMut(usize, ElectricCurrent),
    ) -> Result<(), SolveError> {
        for e in &self.forest.tree {
            let p = &c.parts[e.source];
            let offset =
                self.offsets[e.parent] + p.step_to(e.child, voltage(p, e.source, excitation));
            if !offset.as_base().is_finite() {
                return Err(SolveError::Arithmetic);
            }
            self.offsets[e.child] = offset;
        }
        for edge in &c.branches {
            let (a, b) = (self.forest.group[edge.from], self.forest.group[edge.to]);
            if a != b {
                let offset_current =
                    edge.conductance * (self.offsets[edge.from] - self.offsets[edge.to]);
                inject(a, -offset_current);
                inject(b, offset_current);
            }
        }
        Ok(())
    }
}
