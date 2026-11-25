//! Normalize KCL rows and branch-current unknowns at the numerical boundary.
mod coefficients;
mod control;
mod operator;
mod rank;
mod rhs;
use super::{MnaError, linear::Host};
use crate::{Circuit, ElectricCurrent, ElectricPotential, constraints};
use control::{Couplings, Signal};
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
    controls: Couplings,
}

impl System {
    pub fn new(c: &Circuit, max_order: usize) -> Result<Self, MnaError> {
        let count = c.parts.iter().filter(|p| c.voltage_output(p)).count();
        let order = c.names.len() - 1 + count;
        if order > max_order {
            return Err(MnaError::DirectOrder {
                order,
                limit: max_order,
            });
        }
        if !c.requires_mna()
            && let Some(n) = constraints::floating_node(c)
        {
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
        let mut source_units = Vec::with_capacity(count);
        for (index, p) in c.parts.iter().enumerate() {
            if c.voltage_output(p) {
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
        let controls = Couplings::prepare(c, &mut units, &source_units)?;
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
        let system = Self {
            controls,
            resistors,
            sources,
            units,
        };
        if c.requires_mna() {
            rank::verify(&system)?;
        }
        Ok(system)
    }

    pub fn node_count(&self) -> usize {
        self.units.len()
    }
}
