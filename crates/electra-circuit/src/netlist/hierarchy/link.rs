//! Postorder validation also determines which formal pins lead to physical devices.
use super::{
    source::Deck,
    template::{Operation, Size, Template},
};
use crate::FileError;

pub(super) fn qualify(models: &mut [Template<'_>], deck: &Deck) -> Result<(), FileError> {
    let mut state = vec![0u8; models.len()];
    let mut stack = Vec::new();
    for root in 0..models.len() {
        if state[root] != 0 {
            continue;
        }
        stack.push((root, 0));
        state[root] = 1;
        while let Some(&mut (index, ref mut cursor)) = stack.last_mut() {
            let model = &models[index];
            if let Some(card) = model.cards.get(*cursor) {
                *cursor += 1;
                if let Operation::Instance { target, .. } = card.op {
                    match state[target] {
                        0 => {
                            state[target] = 1;
                            stack.push((target, 0));
                        }
                        1 => {
                            let path = stack
                                .iter()
                                .map(|(i, _)| models[*i].name)
                                .collect::<Vec<_>>()
                                .join(" -> ");
                            return Err(deck.error(
                                card.at,
                                format!(
                                    "{}: recursive subcircuit {path} -> {}",
                                    card.name, models[target].name
                                ),
                            ));
                        }
                        _ => {}
                    }
                }
            } else {
                let (pins, size) = summarize(index, models, deck)?;
                models[index].used_pins = pins;
                models[index].size = size;
                state[index] = 2;
                stack.pop();
            }
        }
    }
    Ok(())
}

fn summarize(
    index: usize,
    models: &mut [Template],
    deck: &Deck,
) -> Result<(Vec<bool>, Option<Size>), FileError> {
    // Bind only used formal pins. An unused external pin must not create a floating unknown.
    let mut used = std::mem::take(&mut models[index].used_pins);
    let model = &models[index];
    let mut mark = |node: usize| {
        if node > 0 && node <= used.len() {
            used[node - 1] = true;
        }
    };
    let mut size = model.size;
    for card in &model.cards {
        match &card.op {
            Operation::Device { nodes, input, .. } => {
                nodes.iter().copied().for_each(&mut mark);
                if let crate::netlist::device::Input::Potential(pair) = input {
                    pair.iter().copied().for_each(&mut mark);
                }
            }
            Operation::Instance { target, pins, .. } => {
                for (&node, &connected) in model.pins[pins.clone()]
                    .iter()
                    .zip(&models[*target].used_pins)
                {
                    if connected {
                        mark(node);
                    }
                }
                size = size.and_then(|s| s.checked_add(models[*target].size?));
                if index == 0 && size.is_none() {
                    return Err(deck.error(
                        card.at,
                        format!(
                            "{}: expanded circuit size exceeds addressable storage",
                            card.name
                        ),
                    ));
                }
            }
        }
    }
    Ok((used, size))
}
