use super::{Circuit, ParseError, builder::Builder, control, fold, value::Unit};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

pub(super) fn parse(text: &str) -> Result<Circuit, ParseError> {
    let mut builder = Builder::new();
    let mut nodes = HashMap::from([(Cow::Borrowed("0"), 0)]);
    let mut names = HashSet::new();
    let mut ended = false;
    let mut directive = None;
    for (line, raw) in text.lines().enumerate() {
        let line = line + 1;
        let mut fields = raw.split_whitespace();
        let Some(name) = fields.next() else { continue };
        if name.starts_with('*') {
            continue;
        }
        if ended {
            return Err(ParseError::new(line, "unexpected content after .end"));
        }
        if raw.trim().eq_ignore_ascii_case(".end") {
            ended = true;
            continue;
        }
        if raw.trim().eq_ignore_ascii_case(".op") {
            continue;
        }
        if name.eq_ignore_ascii_case(".dc") {
            if directive.replace((line, raw)).is_some() {
                return Err(ParseError::new(line, "duplicate .dc directive"));
            }
            continue;
        }
        let (Some(a), Some(b), Some(token), None) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return Err(ParseError::new(
                line,
                "expected a four-field R, I or independent V card",
            ));
        };
        if !names.insert(fold(name)) {
            return Err(ParseError::new(line, "duplicate element name"));
        }
        let err = |msg| ParseError::new(line, msg);
        let value = Unit::device(name)
            .and_then(|u| u.value(token))
            .map_err(err)?;
        let [a, b] = [a, b].map(|label| {
            let key = fold(label);
            if let Some(&id) = nodes.get(key.as_ref()) {
                return id;
            }
            let id = builder.node(label.into());
            nodes.insert(key, id);
            id
        });
        builder.emit(name.into(), a, b, value).map_err(err)?;
    }
    drop((names, nodes));
    let mut circuit = builder.finish(
        || ParseError::new(1, "empty circuit"),
        |source, reason| {
            let line = text
                .lines()
                .position(|raw| {
                    raw.split_whitespace()
                        .next()
                        .is_some_and(|w| w.eq_ignore_ascii_case(source))
                })
                .unwrap()
                + 1;
            ParseError::new(line, format!("{source}: {reason}"))
        },
    )?;
    if let Some((line, card)) = directive {
        circuit.sweep = Some(control::parse_card(&circuit, line, card)?);
    }
    Ok(circuit)
}
