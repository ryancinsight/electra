use super::value::Value;
use crate::{
    Circuit, ElectricConductance, ElectricalImpedance, Element, Part, Resistor,
    constraints::Constraints,
};

pub(super) struct Builder {
    nodes: Vec<String>,
    branches: Vec<Resistor>,
    parts: Vec<Part>,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            nodes: vec!["0".into()],
            branches: Vec::new(),
            parts: Vec::new(),
        }
    }
    pub fn reserve(
        &mut self,
        parts: usize,
        resistors: usize,
    ) -> Result<(), std::collections::TryReserveError> {
        self.parts.try_reserve_exact(parts)?;
        self.branches.try_reserve_exact(resistors)
    }
    pub fn node(&mut self, name: String) -> usize {
        let index = self.nodes.len();
        self.nodes.push(name);
        index
    }
    pub fn emit(
        &mut self,
        name: String,
        positive: usize,
        negative: usize,
        value: Value,
    ) -> Result<(), String> {
        if positive == negative {
            return Err(format!("{name}: element endpoints must differ"));
        }
        let element = match value {
            Value::Resistance(r) => {
                let g: ElectricConductance = r.reciprocal();
                if r <= ElectricalImpedance::default() || !g.as_base().is_finite() {
                    return Err(format!(
                        "{name}: resistance must have finite positive conductance"
                    ));
                }
                let branch = self.branches.len();
                self.branches.push(Resistor {
                    from: positive,
                    to: negative,
                    conductance: g,
                });
                Element::Resistor { branch }
            }
            Value::Voltage(volts) => Element::Voltage { volts },
            Value::Current(amperes) => Element::Current { amperes },
            Value::Time(seconds) => {
                return Err(format!(
                    "{name}: time ({} s) cannot drive a DC primitive",
                    seconds.into_base()
                ));
            }
        };
        self.parts.push(Part {
            name,
            positive,
            negative,
            element,
        });
        Ok(())
    }
    pub fn finish<E>(
        self,
        empty: impl FnOnce() -> E,
        locate: impl FnOnce(&str, String) -> E,
    ) -> Result<Circuit, E> {
        if self.parts.is_empty() {
            return Err(empty());
        }
        let constraints = Constraints::compile(self.nodes.len(), &self.parts, &self.branches)
            .map_err(|e| locate(&self.parts[e.source].name, e.reason.to_string()))?;
        Ok(Circuit {
            names: self.nodes,
            parts: self.parts,
            branches: self.branches,
            constraints,
            sweep: None,
        })
    }
}
