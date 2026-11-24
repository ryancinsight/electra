//! Normalize KCL rows and branch-current unknowns at the numerical boundary.
mod rhs;
use super::{MnaError, linear::Host};
use crate::{Circuit, ElectricCurrent, ElectricPotential, Element, constraints};
use athena_core::LinearOperator;
pub(super) use rhs::Drive;
pub(super) const VOLT: ElectricPotential = ElectricPotential::from_base(1.);

fn normalized(current: ElectricCurrent, unit: ElectricCurrent) -> Result<f64, MnaError> {
    let value = (current / unit).into_base();
    if !value.is_finite() || (value == 0. && current != ElectricCurrent::default()) {
        return Err(MnaError::Arithmetic);
    }
    Ok(value)
}

struct Stamp {
    a: usize,
    b: usize,
    at_a: f64,
    at_b: f64,
}
impl Stamp {
    fn new(
        a: usize,
        b: usize,
        current: ElectricCurrent,
        units: &[ElectricCurrent],
    ) -> Result<Self, MnaError> {
        let coefficient = |n: usize| {
            if n == 0 {
                Ok(0.)
            } else {
                normalized(current, units[n])
            }
        };
        Ok(Self {
            a,
            b,
            at_a: coefficient(a)?,
            at_b: coefficient(b)?,
        })
    }
    #[inline]
    fn inject(&self, amount: f64, out: &mut [f64]) {
        out[self.a] += self.at_a * amount;
        out[self.b] -= self.at_b * amount;
    }
}

pub(super) struct Source {
    pub part: usize,
    pub current_unit: ElectricCurrent,
    stamp: Stamp,
}

pub(super) struct System {
    resistors: Vec<Stamp>,
    pub sources: Vec<Source>,
    units: Vec<ElectricCurrent>,
}

impl System {
    pub fn new(c: &Circuit) -> Result<Self, MnaError> {
        if let Some(n) = constraints::floating_node(c) {
            return Err(MnaError::Floating(n));
        }
        let mut units = vec![ElectricCurrent::default(); c.names.len()];
        for r in &c.branches {
            let amps = r.conductance * VOLT;
            for n in [r.from, r.to] {
                if n != 0 {
                    units[n] = units[n].max(amps);
                }
            }
        }
        // Choose all branch units from the passive network before changing any row units.
        let count = c
            .parts
            .iter()
            .filter(|p| matches!(p.element, Element::Voltage { .. }))
            .count();
        let mut source_units = Vec::with_capacity(count);
        for (index, p) in c.parts.iter().enumerate() {
            if matches!(p.element, Element::Voltage { .. }) {
                let mut scale = units[p.positive].max(units[p.negative]);
                if scale == ElectricCurrent::default() {
                    scale = ElectricCurrent::from_base(1.);
                }
                source_units.push((index, scale));
            }
        }
        for &(index, scale) in &source_units {
            let p = &c.parts[index];
            for n in [p.positive, p.negative] {
                if n != 0 {
                    units[n] = units[n].max(scale);
                }
            }
        }
        let mut resistors = Vec::with_capacity(c.branches.len());
        for r in &c.branches {
            resistors.push(Stamp::new(r.from, r.to, r.conductance * VOLT, &units)?);
        }
        let mut sources = Vec::with_capacity(count);
        for (part, current_unit) in source_units {
            let p = &c.parts[part];
            sources.push(Source {
                part,
                current_unit,
                stamp: Stamp::new(p.positive, p.negative, current_unit, &units)?,
            });
        }
        Ok(Self {
            resistors,
            sources,
            units,
        })
    }

    pub fn node_count(&self) -> usize {
        self.units.len()
    }
}

impl LinearOperator<Host> for System {
    fn dimension(&self) -> usize {
        self.units.len() + self.sources.len()
    }
    fn apply(&self, _: &Host, x: &[f64], out: &mut [f64]) -> Result<(), &'static str> {
        if x.len() != self.dimension() || out.len() != x.len() {
            return Err("operator dimension mismatch");
        }
        out.fill(0.);
        for r in &self.resistors {
            r.inject(x[r.a] - x[r.b], out);
        }
        for (j, s) in self.sources.iter().enumerate() {
            let slot = self.units.len() + j;
            s.stamp.inject(x[slot], out);
            out[slot] = x[s.stamp.a] - x[s.stamp.b];
        }
        // One pinned zero slot lets every edge use original node indices.
        out[0] = x[0];
        Ok(())
    }
}
