mod declarations;
mod expand;
mod link;
mod parameters;
mod source;
mod template;

use super::{Circuit, FileError, control};
use std::path::Path;

pub(super) fn parse(text: &str, path: Option<&Path>) -> Result<Circuit, FileError> {
    let (deck, cards) = source::Deck::load(text, path)?;
    let definitions = declarations::Definitions::read(&deck, cards)?;
    let sweep = definitions.sweep;
    let mut models = definitions
        .blocks
        .iter()
        .map(|block| template::Template::compile(block, &definitions, &deck))
        .collect::<Result<Vec<_>, _>>()?;
    drop(definitions);
    link::qualify(&mut models, &deck)?;
    let mut circuit = expand::build(&models, &deck)?;
    if let Some(s) = sweep {
        circuit.sweep = Some(
            control::parse_card(&circuit, s.at.line, deck.text(s))
                .map_err(|e| deck.error(s.at, e.message))?,
        );
    }
    Ok(circuit)
}
