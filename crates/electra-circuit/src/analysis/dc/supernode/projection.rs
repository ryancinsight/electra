use super::Supernodes;
use crate::analysis::dc::{Excitation, Point, voltage};
use crate::{Circuit, DcResult, ElectricCurrent, ElectricPotential, SolveError, SolveOptions};

impl Supernodes<'_> {
    pub(in crate::analysis::dc) fn project(
        &mut self,
        c: &Circuit,
        point: &Point,
        excitation: Excitation,
        options: SolveOptions,
        output: &mut DcResult<'_>,
    ) -> Result<Option<ElectricCurrent>, SolveError> {
        for (node, (_, v)) in output.volts.iter_mut().enumerate() {
            *v = point.potentials[self.forest.group[node]] + self.offsets[node];
            if !v.as_base().is_finite() {
                return Err(SolveError::Arithmetic);
            }
        }
        self.imbalance.fill(ElectricCurrent::default());
        for (index, ((_, out), p)) in output.amperes.iter_mut().zip(&c.parts).enumerate() {
            if let Some(current) =
                p.passive_current(c, |node| output.volts[node].1, index, excitation)
            {
                *out = current;
                self.imbalance[p.positive] += current;
                self.imbalance[p.negative] -= current;
            }
        }
        for edge in self.forest.tree.iter().rev() {
            let p = &c.parts[edge.source];
            let applied = voltage(p, edge.source, excitation);
            let readback = output.volts[p.positive].1 - output.volts[p.negative].1;
            if !readback.as_base().is_finite()
                || (applied != ElectricPotential::default()
                    && readback == ElectricPotential::default())
                || (readback - applied).abs()
                    > ElectricPotential::from_base(1e-12) + applied.abs() * 1e-9
            {
                return Err(SolveError::Arithmetic);
            }
            let leaving = self.imbalance[edge.child];
            if !leaving.as_base().is_finite() {
                return Err(SolveError::Arithmetic);
            }
            output.amperes[edge.source].1 = if p.positive == edge.child {
                -leaving
            } else {
                leaving
            };
            self.imbalance[edge.parent] += leaving;
        }
        // After leaf-to-root reaction recovery only the tree roots can have a
        // nonzero KCL residual. Most well-conditioned points pass the absolute
        // check alone, so incident-current scales are needed only on demand.
        let mut max = ElectricCurrent::default();
        for &root in &self.forest.roots {
            let r = self.imbalance[root];
            if !r.as_base().is_finite() {
                return Err(SolveError::Arithmetic);
            }
            if root != 0 {
                max = max.max(r.abs());
            }
        }
        output.residual_amperes = max;
        output.sweeps = point.sweeps;
        if max <= options.absolute {
            return Ok(None);
        }
        self.scale.fill(ElectricCurrent::default());
        for (p, &(_, current)) in c.parts.iter().zip(&output.amperes) {
            for node in [p.positive, p.negative] {
                let group = self.forest.group[node];
                if self.forest.roots[group] == node {
                    self.scale[group] += current.abs();
                }
            }
        }
        let mut accepted = true;
        for (group, &root) in self.forest.roots.iter().enumerate().skip(1) {
            let scale = self.scale[group];
            if !scale.as_base().is_finite() {
                return Err(SolveError::Arithmetic);
            }
            accepted &= self.imbalance[root].abs() <= options.absolute + scale * options.relative;
        }
        Ok((!accepted).then_some(max))
    }
}
