//! Iterative elaboration. Sibling instances reuse node and parameter scratch space.
use super::{
    source::{Deck, Loc},
    template::{Operation, Template},
};
use crate::{
    Circuit, FileError,
    netlist::{builder::Builder, value::Value},
};

fn qualified(prefix: &str, label: &str) -> String {
    let mut name = String::with_capacity(prefix.len() + label.len());
    name.push_str(prefix);
    name.push_str(label);
    name
}

struct Frame {
    model: usize,
    next: usize,
    node_base: usize,
    param_base: usize,
    parent_path_len: usize,
}
struct Expansion<'a> {
    builder: Builder,
    stack: Vec<Frame>,
    slots: Vec<usize>,
    params: Vec<Value>,
    path: String,
    models: &'a [Template<'a>],
}

impl<'a> Expansion<'a> {
    fn push(&mut self, model: usize, parent_path_len: usize) {
        let node_base = self.slots.len();
        let param_base = self.params.len();
        self.slots
            .resize(node_base + self.models[model].nodes.len(), usize::MAX);
        self.slots[node_base] = 0;
        self.params.extend_from_slice(&self.models[model].defaults);
        self.stack.push(Frame {
            model,
            next: 0,
            node_base,
            param_base,
            parent_path_len,
        });
    }

    fn node(&mut self, node: usize, base: usize, label: &str) -> usize {
        let slot = &mut self.slots[base + node];
        if *slot == usize::MAX {
            *slot = self.builder.node(qualified(&self.path, label));
        }
        *slot
    }

    fn run(&mut self, deck: &Deck) -> Result<(), FileError> {
        self.push(0, 0);
        while let Some(frame) = self.stack.last_mut() {
            let model = &self.models[frame.model];
            let Some(card) = model.cards.get(frame.next) else {
                self.slots.truncate(frame.node_base);
                self.params.truncate(frame.param_base);
                self.path.truncate(frame.parent_path_len);
                self.stack.pop();
                continue;
            };
            frame.next += 1;
            let (base, params) = (frame.node_base, frame.param_base);
            match &card.op {
                Operation::Device {
                    nodes: [a, b],
                    value,
                } => {
                    let a = self.node(*a, base, model.nodes[*a]);
                    let b = self.node(*b, base, model.nodes[*b]);
                    self.builder
                        .emit(
                            qualified(&self.path, card.name),
                            a,
                            b,
                            value.get(&self.params[params..]),
                        )
                        .map_err(|msg| deck.error(card.at, msg))?;
                }
                Operation::Instance {
                    target,
                    pins,
                    overrides,
                } => {
                    if self.models[*target]
                        .size
                        .is_some_and(|size| size.parts == 0)
                    {
                        continue;
                    }
                    let parent_len = self.path.len();
                    let child_slots = self.slots.len();
                    let child_params = self.params.len();
                    self.push(*target, parent_len);
                    for (pin, &local) in model.pins[pins.clone()].iter().enumerate() {
                        if self.models[*target].used_pins[pin] {
                            let id = self.node(local, base, model.nodes[local]);
                            self.slots[child_slots + pin + 1] = id;
                        }
                    }
                    for &(slot, value) in &model.overrides[overrides.clone()] {
                        self.params[child_params + slot] =
                            value.get(&self.params[params..child_params]);
                    }
                    self.path.push_str(card.name);
                    self.path.push('/');
                }
            }
        }
        Ok(())
    }
}

fn origin(models: &[Template], qualified: &str) -> Loc {
    let mut model = &models[0];
    for name in qualified.split('/') {
        let card = model
            .cards
            .iter()
            .find(|c| c.name == name)
            .expect("qualified path came from compiled cards");
        match card.op {
            Operation::Instance { target, .. } => model = &models[target],
            Operation::Device { .. } => return card.at,
        }
    }
    unreachable!("qualified path ends at a primitive")
}

pub(super) fn build(models: &[Template], deck: &Deck) -> Result<Circuit, FileError> {
    let mut expansion = Expansion {
        builder: Builder::new(),
        stack: Vec::new(),
        slots: Vec::new(),
        params: Vec::new(),
        path: String::new(),
        models,
    };
    let size = models[0]
        .size
        .expect("root physical size checked during linking");
    expansion
        .builder
        .reserve(size.parts, size.resistors)
        .map_err(|error| {
            deck.error(
                Loc { file: 0, line: 1 },
                format!(
                    "cannot reserve storage for {} expanded parts: {error}",
                    size.parts
                ),
            )
        })?;
    expansion.run(deck)?;
    expansion.builder.finish(
        || deck.error(Loc { file: 0, line: 1 }, "empty circuit"),
        |name, reason| deck.error(origin(models, name), format!("{name}: {reason}")),
    )
}
