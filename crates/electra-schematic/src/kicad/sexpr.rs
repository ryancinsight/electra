//! Borrow complete atoms or expressions, retaining source spelling and spacing.
use std::io::{Error, ErrorKind, Result};
pub(super) fn invalid(reason: &str) -> Error {
    Error::new(ErrorKind::InvalidData, reason)
}

pub(super) fn next<'a>(tail: &mut &'a str) -> Result<Option<&'a str>> {
    let source = tail.trim_start_matches(|c: char| c.is_ascii_whitespace());
    if source.is_empty() {
        *tail = source;
        return Ok(None);
    }
    let bytes = source.as_bytes();
    let mut i = 0;
    let mut depth = 0usize;
    loop {
        match bytes[i] {
            b'(' => {
                depth += 1;
                i += 1;
            }
            b')' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| invalid("unexpected closing parenthesis"))?;
                i += 1;
            }
            b'"' => {
                i += 1;
                loop {
                    match bytes.get(i) {
                        Some(b'\\') => i += 2,
                        Some(b'"') => {
                            i += 1;
                            break;
                        }
                        Some(_) => i += 1,
                        None => return Err(invalid("unterminated string")),
                    }
                }
            }
            b if b.is_ascii_whitespace() => i += 1,
            _ => {
                i += 1;
                while i < bytes.len()
                    && !bytes[i].is_ascii_whitespace()
                    && !b"()\"".contains(&bytes[i])
                {
                    i += 1;
                }
            }
        }
        if depth == 0 {
            break;
        }
        if i >= bytes.len() {
            return Err(invalid("unterminated expression"));
        }
    }
    *tail = &source[i..];
    Ok(Some(&source[..i]))
}

pub(super) fn split(expr: &str) -> Result<(&str, &str)> {
    let mut inner = expr
        .strip_prefix('(')
        .and_then(|v| v.strip_suffix(')'))
        .ok_or_else(|| invalid("expected list"))?;
    let tag = next(&mut inner)?.ok_or_else(|| invalid("missing list tag"))?;
    Ok((tag, inner))
}

pub(super) fn field<'a>(body: &'a str, tag: &str) -> Result<Option<&'a str>> {
    let mut tail = body;
    let mut found = None;
    while let Some(node) = next(&mut tail)? {
        if node.starts_with('(') {
            let (name, values) = split(node)?;
            if name == tag && found.replace(values).is_some() {
                return Err(invalid("ambiguous field"));
            }
        }
    }
    Ok(found)
}
