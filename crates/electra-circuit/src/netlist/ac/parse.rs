use super::{AcDeck, ParseError};
use crate::netlist::{fold, number::magnitude};
use electra_ac::{
    Capacitance, ElectricPotential, ElectricalImpedance, Inductance,
    nodal::{PassiveBranch, PassiveElement},
};
use std::collections::{HashMap, HashSet};

/// Parse flat R/C/L cards and ground-referenced `Vname p n AC magnitude [degrees]`
/// sources. Node and element names are ASCII case-insensitive. Values accept
/// the same SI scale factors as DC decks. Frequency is supplied at assembly.
pub fn parse_ac(text: &str) -> Result<AcDeck, ParseError> {
    let mut ids = HashMap::from([(fold("0"), 0)]);
    let mut devices = HashSet::new();
    let mut deck = AcDeck {
        names: vec!["0".into()],
        boundary: vec![Some(ElectricPotential::default())],
        branches: Vec::new(),
    };
    let mut ended = false;
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let error = |s| ParseError::new(line, s);
        let mut fields = raw.split_whitespace();
        let Some(name) = fields.next() else { continue };
        if name.starts_with('*') {
            continue;
        }
        if ended {
            return Err(error("unexpected content after .end"));
        }
        if name.eq_ignore_ascii_case(".end") && fields.clone().next().is_none() {
            ended = true;
            continue;
        }
        let kind = name.as_bytes()[0].to_ascii_uppercase();
        if !matches!(kind, b'R' | b'C' | b'L' | b'V') {
            return Err(error(
                "expected R, C, L, ground-referenced AC voltage source or .end",
            ));
        }
        if !devices.insert(fold(name)) {
            return Err(error("duplicate element name"));
        }
        let mut node = || {
            let label = fields.next().ok_or_else(|| error("missing endpoint"))?;
            let id = *ids.entry(fold(label)).or_insert_with(|| {
                let id = deck.names.len();
                deck.names.push(label.to_owned());
                deck.boundary.push(None);
                id
            });
            Ok::<_, ParseError>(id)
        };
        let (a, b) = (node()?, node()?);
        if kind == b'V' {
            super::source::prescribe(a, b, &mut fields, line, &mut deck.boundary)?;
        } else {
            let x = value(fields.next(), line)?;
            let element = match kind {
                b'R' => PassiveElement::resistor(ElectricalImpedance::from_base(x)),
                b'C' => PassiveElement::capacitor(Capacitance::from_base(x)),
                _ => PassiveElement::inductor(Inductance::from_base(x)),
            }
            .map_err(|e| ParseError::new(line, e.to_string()))?;
            deck.branches
                .push((line, PassiveBranch::new(a, b, element)));
        }
        if fields.next().is_some() {
            return Err(error("unexpected trailing field"));
        }
    }
    if deck.branches.is_empty() {
        return Err(ParseError::new(1, "AC deck requires a passive branch"));
    }
    Ok(deck)
}

pub(super) fn value(token: Option<&str>, line: usize) -> Result<f64, ParseError> {
    token
        .and_then(magnitude)
        .ok_or_else(|| ParseError::new(line, "invalid or missing numeric value"))
}
