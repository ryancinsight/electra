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

fn node(names: &mut Vec<String>, ids: &mut HashMap<String, usize>, name: &str) -> usize {
    *ids.entry(name.to_ascii_lowercase()).or_insert_with(|| {
        let index = names.len();
        names.push(name.to_owned());
        index
    })
}

/// Parse four-field R, I and independent V cards. Values use SI base
/// units, decimal/scientific notation and optional SPICE scale factors. Names are ASCII-case-insensitive.
/// Blank lines, whole-line '*' comments, '.op', a single '.dc source start stop step'
/// and a final '.end' are accepted. A sweep may precede its source.
pub fn parse(text: &str) -> Result<Circuit, ParseError> {
    let mut node_names = vec!["0".into()];
    let mut branches = Vec::new();
    let mut parts = Vec::new();
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
            return Err(err("expected a four-field R, I or independent V card"));
        };
        if !names.insert(name.to_ascii_lowercase()) {
            return Err(err("duplicate element name"));
        }
        let [a, b] = [p, n].map(|s| node(&mut node_names, &mut nodes, s));
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
                let branch = branches.len();
                branches.push(Branch {
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
            b'V' => Element::Voltage {
                volts: ElectricPotential::from_base(value),
            },
            _ => return Err(err("unsupported element type")),
        };
        parts.push(Part {
            name: name.into(),
            positive: a,
            negative: b,
            element,
        });
    }
    if parts.is_empty() {
        return Err(ParseError {
            line: 1,
            message: "empty circuit".into(),
        });
    }
    drop((nodes, names));
    let constraints = crate::constraints::Constraints::compile(node_names.len(), &parts, &branches)
        .map_err(|error| {
            let source = &parts[error.source].name;
            // Successful parses keep no per-element line table. Source names are unique.
            let line = text
                .lines()
                .position(|raw| {
                    raw.split_whitespace()
                        .next()
                        .is_some_and(|name| name.eq_ignore_ascii_case(source))
                })
                .expect("compiled source came from this deck")
                + 1;
            ParseError {
                line,
                message: format!("{source}: {}", error.reason),
            }
        })?;
    let mut circuit = Circuit {
        names: node_names,
        parts,
        branches,
        constraints,
        sweep: None,
    };
    if let Some((line, card)) = directive {
        circuit.sweep = Some(control::parse_card(&circuit, line, card)?);
    }
    Ok(circuit)
}
