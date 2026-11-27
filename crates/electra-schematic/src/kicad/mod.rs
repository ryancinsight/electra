//! Lossless KiCad 9 property editing. Source text owns all unmodeled fields.
//! Schematic symbols are addressed by UUID and library symbols by entry name.
mod sexpr;
use sexpr::{field, invalid, next, split};
use std::io::{Result, Write};

/// Supported KiCad file schemas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Schematic dated 20250114.
    Schematic,
    /// Symbol library dated 20241209.
    Symbols,
}
/// A validated root and version, borrowing the entire original document.
pub struct Document<'a> {
    source: &'a str,
    body: &'a str,
    kind: Kind,
}

fn quoted(value: &str) -> Result<String> {
    if value.chars().any(char::is_control) {
        return Err(invalid("control character in edited field"));
    }
    Ok(format!(
        "\"{}\"",
        value.replace('\\', "\\\\").replace('"', "\\\"")
    ))
}

impl<'a> Document<'a> {
    /// Validate the root, balanced expressions, quoted strings and schema version.
    pub fn parse(source: &'a str) -> Result<Self> {
        let mut tail = source;
        let root = next(&mut tail)?.ok_or_else(|| invalid("empty document"))?;
        if next(&mut tail)?.is_some() {
            return Err(invalid("trailing data"));
        }
        let (tag, body) = split(root)?;
        let (kind, version) = match tag {
            "kicad_sch" => (Kind::Schematic, "20250114"),
            "kicad_symbol_lib" => (Kind::Symbols, "20241209"),
            _ => return Err(invalid("unsupported KiCad document")),
        };
        if field(body, "version")?.map(str::trim) != Some(version) {
            return Err(invalid("unsupported KiCad version"));
        }
        Ok(Self { source, body, kind })
    }
    /// Recognized interchange schema.
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// Write an exact copy, including unknown fields.
    pub fn write(&self, mut out: impl Write) -> Result<()> {
        out.write_all(self.source.as_bytes())
    }

    /// Replace one existing quoted property on one symbol. All other bytes
    /// pass through unchanged. Ambiguous selectors fail before writing.
    pub fn write_property(
        &self,
        id: &str,
        key: &str,
        value: &str,
        mut out: impl Write,
    ) -> Result<()> {
        let id = quoted(id)?;
        let key = quoted(key)?;
        let replacement = quoted(value)?;
        let mut tail = self.body;
        let mut symbol = None;
        while let Some(node) = next(&mut tail)? {
            if !node.starts_with('(') {
                continue;
            }
            let (tag, mut body) = split(node)?;
            if tag != "symbol" {
                continue;
            }
            let matches = match self.kind {
                Kind::Schematic => field(body, "uuid")?.is_some_and(|v| v.trim() == id),
                Kind::Symbols => next(&mut body)? == Some(id.as_str()),
            };
            if matches && symbol.replace(body).is_some() {
                return Err(invalid("ambiguous symbol"));
            }
        }
        let mut body = symbol.ok_or_else(|| invalid("symbol not found"))?;
        let mut target = None;
        while let Some(node) = next(&mut body)? {
            if !node.starts_with('(') {
                continue;
            }
            let (tag, mut args) = split(node)?;
            if tag == "property" && next(&mut args)? == Some(key.as_str()) {
                let token = next(&mut args)?
                    .filter(|s| s.starts_with('"'))
                    .ok_or_else(|| invalid("expected quoted property"))?;
                if target.replace(token).is_some() {
                    return Err(invalid("ambiguous property"));
                }
            }
        }
        let token = target.ok_or_else(|| invalid("property not found"))?;
        let start = token.as_ptr() as usize - self.source.as_ptr() as usize;
        out.write_all(&self.source.as_bytes()[..start])?;
        out.write_all(replacement.as_bytes())?;
        out.write_all(&self.source.as_bytes()[start + token.len()..])
    }
}
