use super::{
    parameters::{Parameter, declare, identifier},
    source::{Deck, Span},
};
use crate::netlist::{FileError, fold};
use std::collections::HashMap;

pub(super) struct Block<'a> {
    pub name: &'a str,
    pub pins: Vec<&'a str>,
    pub params: Vec<Parameter<'a>>,
    pub formals: usize,
    pub cards: Vec<Span>,
}
impl<'a> Block<'a> {
    fn new(name: &'a str) -> Self {
        Self {
            name,
            pins: Vec::new(),
            params: Vec::new(),
            formals: 0,
            cards: Vec::new(),
        }
    }
    fn header(mut words: std::str::SplitWhitespace<'a>) -> Result<Self, String> {
        let name = words.next().ok_or("missing .subckt name")?;
        identifier(name)?;
        let mut block = Self::new(name);
        let mut parameters = false;
        for word in words {
            if word.eq_ignore_ascii_case("params:") && !parameters {
                parameters = true;
                continue;
            }
            if parameters {
                declare(&mut block.params, word)?;
            } else {
                identifier(word)?;
                if word == "0" || block.pins.iter().any(|p| p.eq_ignore_ascii_case(word)) {
                    return Err(format!("{name}: duplicate or ground formal pin {word}"));
                }
                block.pins.push(word);
            }
        }
        block.formals = block.params.len();
        Ok(block)
    }
}

pub(super) struct Definitions<'a> {
    pub blocks: Vec<Block<'a>>,
    pub ids: HashMap<String, usize>,
    pub sweep: Option<Span>,
}
impl<'a> Definitions<'a> {
    pub fn read(deck: &'a Deck, cards: Vec<Span>) -> Result<Self, FileError> {
        let mut d = Self {
            blocks: vec![Block::new("<root>")],
            ids: HashMap::new(),
            sweep: None,
        };
        let mut scope = 0;
        for span in cards {
            let mut words = deck.text(span).split_whitespace();
            let word = words.next().unwrap();
            let err = |msg: String| deck.error(span.at, msg);
            if word.eq_ignore_ascii_case(".subckt") {
                let b = Block::header(words).map_err(err)?;
                scope = d.blocks.len();
                if d.ids.insert(fold(b.name).into_owned(), scope).is_some() {
                    return Err(err(format!("duplicate subcircuit {}", b.name)));
                }
                d.blocks.push(b);
            } else if word.eq_ignore_ascii_case(".ends") {
                if words
                    .next()
                    .is_some_and(|name| !name.eq_ignore_ascii_case(d.blocks[scope].name))
                    || words.next().is_some()
                {
                    return Err(err(format!("expected .ends {}", d.blocks[scope].name)));
                }
                scope = 0;
            } else if word.eq_ignore_ascii_case(".param") {
                let mut words = words.peekable();
                if words.peek().is_none() {
                    return Err(err("empty .param declaration".into()));
                }
                for token in words {
                    declare(&mut d.blocks[scope].params, token).map_err(err)?;
                }
            } else if word.eq_ignore_ascii_case(".dc") || word.eq_ignore_ascii_case(".op") {
                if scope != 0 {
                    return Err(err(format!("{word} requires root scope")));
                }
                if word.eq_ignore_ascii_case(".dc") {
                    if d.sweep.replace(span).is_some() {
                        return Err(err("duplicate .dc directive".into()));
                    }
                } else if words.next().is_some() {
                    return Err(err("unexpected fields after .op".into()));
                }
            } else {
                d.blocks[scope].cards.push(span);
            }
        }
        Ok(d)
    }
}
