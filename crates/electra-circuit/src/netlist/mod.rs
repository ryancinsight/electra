//! Read linear DC decks and elaborate typed, hierarchical designs.
pub(crate) mod ac;
mod builder;
mod control;
mod device;
mod error;
mod flat;
mod hierarchy;
mod number;
mod value;

use crate::Circuit;
pub use error::{FileError, ParseError};
use std::{borrow::Cow, fs, path::Path};

fn fold(name: &str) -> Cow<'_, str> {
    if name.bytes().any(|b| b.is_ascii_uppercase()) {
        Cow::Owned(name.to_ascii_lowercase())
    } else {
        Cow::Borrowed(name)
    }
}

fn needs_elaboration(text: &str) -> bool {
    text.lines().any(|raw| {
        let text = raw.trim_start();
        match text.as_bytes().first() {
            Some(b'X' | b'x') => true,
            Some(b'.') => text.split_whitespace().next().is_some_and(|w| {
                [".subckt", ".ends", ".param", ".include"]
                    .iter()
                    .any(|s| w.eq_ignore_ascii_case(s))
            }),
            _ => false,
        }
    })
}

/// Parse an in-memory netlist, with case-insensitive names and globally shared node zero.
/// Supports R/I/V/E/G/F/H, .op, .dc, .end, .subckt/.ends, X instances and typed .param cards.
/// Part values are in base SI units and accept decimal or scientific notation with
/// optional SPICE scale factors. Blank lines and whole-line `*` comments are accepted.
/// Use [`parse_file`] for relative includes.
pub fn parse(text: &str) -> Result<Circuit, ParseError> {
    if needs_elaboration(text) {
        hierarchy::parse(text, None).map_err(|e| e.error)
    } else {
        flat::parse(text)
    }
}

/// Read a root netlist and resolve .include relative to the containing file.
pub fn parse_file(path: impl AsRef<Path>) -> Result<Circuit, FileError> {
    let path = path.as_ref();
    let text = fs::read_to_string(path).map_err(|e| FileError {
        path: path.into(),
        error: ParseError::new(1, e.to_string()),
    })?;
    if needs_elaboration(&text) {
        hierarchy::parse(&text, Some(path))
    } else {
        flat::parse(&text).map_err(|error| FileError {
            path: path.into(),
            error,
        })
    }
}
