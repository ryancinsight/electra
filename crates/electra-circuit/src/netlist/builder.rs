use super::{device::Input, fold, value::Value};
use crate::device::Controlled;
use crate::{
    Circuit, ElectricConductance, ElectricalImpedance, Element, Part, Resistor,
    constraints::Constraints,
};
use std::collections::HashMap;

pub(super) struct Builder {
    nodes: Vec<String>,
    branches: Vec<Resistor>,
    parts: Vec<Part>,
    controlled: Vec<Controlled>,
    // Resolve forward branch references once after elaboration.
    references: Vec<(usize, usize, String)>,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            nodes: vec!["0".into()],
            branches: Vec::new(),
            parts: Vec::new(),
            controlled: Vec::new(),
            references: Vec::new(),
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
        input: Input<usize, String>,
        value: Value,
    ) -> Result<(), String> {
        if positive == negative {
            return Err(format!("{name}: element endpoints must differ"));
        }
        let element = match (input, value) {
            (Input::Independent, Value::Resistance(r)) => {
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
            (Input::Independent, Value::Voltage(volts)) => Element::Voltage { volts },
            (Input::Independent, Value::Current(amperes)) => Element::Current { amperes },
            (_, Value::Time(t)) => {
                return Err(format!(
                    "{name}: time ({} s) cannot drive a DC primitive",
                    t.into_base()
                ));
            }
            (input, value) => {
                let slot = self.controlled.len();
                let law = match (input, value) {
                    (Input::Potential(pair), Value::Gain(k)) => Controlled::VoltageGain(pair, k),
                    (Input::Potential(pair), Value::Conductance(g)) => {
                        Controlled::Transconductance(pair, g)
                    }
                    (Input::Branch(sense), value) => {
                        self.references.push((slot, self.parts.len(), sense));
                        match value {
                            Value::Gain(k) => Controlled::CurrentGain(0, k),
                            Value::Resistance(r) => Controlled::Transresistance(0, r),
                            _ => {
                                return Err(format!("{name}: invalid current-control coefficient"));
                            }
                        }
                    }
                    _ => return Err(format!("{name}: invalid device coefficient")),
                };
                self.controlled.push(law);
                Element::Controlled(slot)
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
        locate: impl Fn(&str, String) -> E,
    ) -> Result<Circuit, E> {
        if self.parts.is_empty() {
            return Err(empty());
        }
        let constraints = if self.controlled.is_empty() {
            Constraints::compile(self.nodes.len(), &self.parts, &self.branches)
        } else {
            Constraints::general(self.nodes.len(), &self.parts)
        }
        .map_err(|e| locate(&self.parts[e.source].name, e.reason.to_string()))?;
        let mut circuit = Circuit {
            names: self.nodes,
            parts: self.parts,
            branches: self.branches,
            constraints,
            sweep: None,
            controlled: self.controlled,
        };
        if !self.references.is_empty() {
            let branches: HashMap<_, _> = circuit
                .parts
                .iter()
                .filter(|p| circuit.voltage_output(p))
                .enumerate()
                .map(|(slot, p)| (fold(&p.name), slot))
                .collect();
            for (law, part, name) in self.references {
                let slot = *branches.get(&fold(&name)).ok_or_else(|| {
                    locate(
                        &circuit.parts[part].name,
                        format!("controlling branch {name} must name a voltage source (V, E or H)"),
                    )
                })?;
                match &mut circuit.controlled[law] {
                    Controlled::CurrentGain(s, _) | Controlled::Transresistance(s, _) => *s = slot,
                    _ => unreachable!("only current controls have pending references"),
                }
            }
        }
        Ok(circuit)
    }
}
