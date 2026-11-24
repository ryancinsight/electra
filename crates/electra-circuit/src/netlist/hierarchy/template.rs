//! Link device and parameter names to compact slots before instantiation.
use super::{
    declarations::{Block, Definitions},
    parameters::{Operand, assignment, identifier},
    source::{Deck, Loc},
};
use crate::netlist::{
    FileError,
    device::{Device, Input},
    fold,
    value::Value,
};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ops::Range,
};

pub(super) enum Operation<'a> {
    Device {
        nodes: [usize; 2],
        value: Operand,
        input: Input<usize, &'a str>,
    },
    Instance {
        target: usize,
        pins: Range<usize>,
        overrides: Range<usize>,
    },
}
pub(super) struct Card<'a> {
    pub name: &'a str,
    pub at: Loc,
    pub op: Operation<'a>,
}

/// Physical storage required by one expanded instance.
#[derive(Clone, Copy, Default)]
pub(super) struct Size {
    pub parts: usize,
    pub resistors: usize,
}
impl Size {
    pub fn checked_add(self, child: Self) -> Option<Self> {
        Some(Self {
            parts: self.parts.checked_add(child.parts)?,
            resistors: self.resistors.checked_add(child.resistors)?,
        })
    }
}

pub(super) struct Template<'a> {
    pub name: &'a str,
    pub nodes: Vec<&'a str>,
    pub defaults: Vec<Value>,
    pub used_pins: Vec<bool>,
    pub size: Option<Size>,
    pub cards: Vec<Card<'a>>,
    pub pins: Vec<usize>,
    pub overrides: Vec<(usize, Operand)>,
}

struct NodeTable<'a> {
    names: Vec<&'a str>,
    ids: HashMap<Cow<'a, str>, usize>,
}
impl<'a> NodeTable<'a> {
    fn intern(&mut self, name: &'a str) -> Result<usize, String> {
        identifier(name)?;
        Ok(*self.ids.entry(fold(name)).or_insert_with(|| {
            let i = self.names.len();
            self.names.push(name);
            i
        }))
    }
}

impl<'a> Template<'a> {
    pub fn compile(
        block: &Block<'a>,
        defs: &Definitions<'a>,
        deck: &'a Deck,
    ) -> Result<Self, FileError> {
        let mut nodes = NodeTable {
            names: vec!["0"],
            ids: HashMap::from([(Cow::Borrowed("0"), 0)]),
        };
        for &pin in &block.pins {
            nodes.intern(pin).expect("validated formal pin");
        }
        let mut names = HashSet::new();
        let mut direct = Size::default();
        let mut template = Self {
            name: block.name,
            nodes: Vec::new(),
            defaults: block.params.iter().map(|p| p.default).collect(),
            used_pins: vec![false; block.pins.len()],
            size: None,
            cards: Vec::with_capacity(block.cards.len()),
            pins: Vec::new(),
            overrides: Vec::new(),
        };
        for &span in &block.cards {
            let mut words = deck.text(span).split_whitespace();
            let name = words.next().unwrap();
            let err = |msg: String| deck.error(span.at, format!("{} / {name}: {msg}", block.name));
            identifier(name).map_err(err)?;
            if !names.insert(fold(name)) {
                return Err(err("duplicate element or instance name".into()));
            }
            let op = if name.as_bytes()[0].eq_ignore_ascii_case(&b'X') {
                template.call(words, block, defs, &mut nodes).map_err(err)?
            } else {
                let card = Device::read(name, words).map_err(err)?;
                let [a, b] = card.ports;
                let ports = [nodes.intern(a).map_err(err)?, nodes.intern(b).map_err(err)?];
                if ports[0] == ports[1] {
                    return Err(err("element endpoints must differ".into()));
                }
                let input = card
                    .input
                    .map(
                        |n| nodes.intern(n),
                        |b| {
                            identifier(b)?;
                            Ok(b)
                        },
                    )
                    .map_err(err)?;
                let value =
                    Operand::bind(card.magnitude, card.unit, &block.params, false).map_err(err)?;
                direct.parts += 1;
                direct.resistors += usize::from(name.as_bytes()[0].eq_ignore_ascii_case(&b'R'));
                Operation::Device {
                    nodes: ports,
                    input,
                    value,
                }
            };
            template.cards.push(Card {
                name,
                at: span.at,
                op,
            });
        }
        // Validate every model, including definitions without an instance.
        for card in &template.cards {
            if let Operation::Device {
                input: Input::Branch(sense),
                ..
            } = card.op
                && (!matches!(sense.as_bytes()[0].to_ascii_uppercase(), b'V' | b'E' | b'H')
                    || !names.contains(&fold(sense)))
            {
                return Err(deck.error(card.at, format!("{} / {}: controlling branch {sense} must name a local voltage source (V, E or H)",block.name,card.name)));
            }
        }
        template.nodes = nodes.names;
        template.size = Some(direct);
        Ok(template)
    }

    fn call(
        &mut self,
        words: std::str::SplitWhitespace<'a>,
        parent: &Block<'a>,
        defs: &Definitions<'a>,
        nodes: &mut NodeTable<'a>,
    ) -> Result<Operation<'a>, String> {
        let positional = words
            .clone()
            .take_while(|w| !w.eq_ignore_ascii_case("params:") && !w.contains('='))
            .count();
        let model = positional
            .checked_sub(1)
            .and_then(|i| words.clone().nth(i))
            .ok_or("missing instance model")?;
        let target = *defs
            .ids
            .get(fold(model).as_ref())
            .ok_or_else(|| format!("unknown subcircuit {model}"))?;
        let child = &defs.blocks[target];
        if positional - 1 != child.pins.len() {
            return Err(format!(
                "{model} expects {} pins, got {}",
                child.pins.len(),
                positional - 1
            ));
        }
        let start = self.pins.len();
        for w in words.clone().take(positional - 1) {
            self.pins.push(nodes.intern(w)?);
        }
        let pins = start..self.pins.len();
        let start = self.overrides.len();
        let mut arguments = words.skip(positional).peekable();
        if arguments
            .peek()
            .is_some_and(|w| w.eq_ignore_ascii_case("params:"))
        {
            arguments.next();
        }
        for arg in arguments {
            let (key, val) = assignment(arg)?;
            let slot = child.params[..child.formals]
                .iter()
                .position(|p| p.name.eq_ignore_ascii_case(key))
                .ok_or_else(|| format!("{model} has no formal parameter {key}"))?;
            if self.overrides[start..].iter().any(|(s, _)| *s == slot) {
                return Err(format!("duplicate override {key}"));
            }
            let operand =
                Operand::bind(val, child.params[slot].default.unit(), &parent.params, true)
                    .map_err(|e| format!("{model}.{key}: {e}"))?;
            self.overrides.push((slot, operand));
        }
        Ok(Operation::Instance {
            target,
            pins,
            overrides: start..self.overrides.len(),
        })
    }
}
