mod control;
mod number;
use super::{
    Circuit, ElectricConductance, ElectricCurrent, ElectricPotential, ElectricalImpedance, Element,
    Part,
};
use electra_network::Branch;
use number::magnitude;
use std::{
    collections::{HashMap, HashSet},
    fmt,
};

/// A line-numbered netlist error.
#[derive(Debug, PartialEq)]
pub struct ParseError {
    /// One-based source line.
    pub line: usize,
    /// Diagnostic describing the invalid card.
    pub message: String,
}
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}
impl std::error::Error for ParseError {}

impl Circuit {
    fn node(&mut self, ids: &mut HashMap<String, usize>, name: &str) -> usize {
        let key = name.to_ascii_lowercase();
        *ids.entry(key).or_insert_with(|| {
            let index = self.names.len();
            self.names.push(name.to_owned());
            self.fixed_volts.push(None);
            index
        })
    }
}

/// Parse four-field R, I and ground-referenced V cards. Values use SI base
/// units, decimal/scientific notation and optional SPICE scale factors. Names are ASCII-case-insensitive.
/// Blank lines, whole-line '*' comments, '.op', a single '.dc source start stop step'
/// and a final '.end' are accepted. A sweep may precede its source.
pub fn parse(text: &str) -> Result<Circuit, ParseError> {
    let mut circuit = Circuit {
        names: vec!["0".into()],
        fixed_volts: vec![Some(ElectricPotential::default())],
        sweep: None,
        branches: vec![],
        parts: vec![],
    };
    let mut nodes = HashMap::from([("0".to_string(), 0)]);
    let mut names = HashSet::new();
    let mut ended = false;
    let mut directive = None;
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        let err = |message: &str| ParseError {
            line: index + 1,
            message: message.to_string(),
        };
        if line.is_empty() || line.starts_with('*') {
            continue;
        }
        if ended {
            return Err(err("unexpected content after .end"));
        }
        if line.eq_ignore_ascii_case(".end") {
            ended = true;
            continue;
        }
        if line.eq_ignore_ascii_case(".op") {
            continue;
        }
        let mut fields = line.split_whitespace();
        if fields
            .clone()
            .next()
            .is_some_and(|word| word.eq_ignore_ascii_case(".dc"))
        {
            if directive.is_some() {
                return Err(err("duplicate .dc directive"));
            }
            directive = Some((index + 1, line));
            continue;
        }
        let (Some(name), Some(p), Some(n), Some(value), None) = (
            fields.next(),
            fields.next(),
            fields.next(),
            fields.next(),
            fields.next(),
        ) else {
            return Err(err(
                "expected a four-field R, I or ground-referenced V card",
            ));
        };
        if !names.insert(name.to_ascii_lowercase()) {
            return Err(err("duplicate element name"));
        }
        let [a, b] = [p, n].map(|s| circuit.node(&mut nodes, s));
        if a == b {
            return Err(err("element endpoints must differ"));
        }
        let value = magnitude(value).ok_or_else(|| {
            err("expected a finite SI value with an optional supported scale factor")
        })?;
        let element = match name.as_bytes()[0].to_ascii_uppercase() {
            b'R' => {
                let resistance = ElectricalImpedance::from_base(value);
                let g: ElectricConductance = resistance.reciprocal();
                if resistance <= ElectricalImpedance::default() || !g.as_base().is_finite() {
                    return Err(err("resistance must have finite positive conductance"));
                }
                let branch = circuit.branches.len();
                circuit.branches.push(Branch {
                    from: a,
                    to: b,
                    conductance: g,
                });
                Element::Resistor { branch }
            }
            b'I' => {
                let value = ElectricCurrent::from_base(value);
                Element::Current { amperes: value }
            }
            b'V' => {
                let value = ElectricPotential::from_base(value);
                if a != 0 && b != 0 {
                    return Err(err("0.1 voltage sources require one endpoint at node 0"));
                }
                let (prescribed, voltage) = crate::prescribed_voltage(a, b, value);
                if circuit.fixed_volts[prescribed].replace(voltage).is_some() {
                    return Err(err(
                        "multiple ideal voltage sources constrain the same node",
                    ));
                }
                Element::Voltage { prescribed }
            }
            _ => return Err(err("unsupported element type")),
        };
        circuit.parts.push(Part {
            name: name.into(),
            positive: a,
            negative: b,
            element,
        });
    }
    if circuit.parts.is_empty() {
        return Err(ParseError {
            line: 1,
            message: "empty circuit".into(),
        });
    }
    if let Some((line, card)) = directive {
        circuit.sweep = Some(control::parse_card(&circuit, line, card)?);
    }
    Ok(circuit)
}
