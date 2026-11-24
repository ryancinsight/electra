//! Source ownership and depth-first, file-relative include expansion.
use super::super::{FileError, ParseError};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Copy, Clone)]
pub(super) struct Loc {
    pub file: usize,
    pub line: usize,
}
#[derive(Copy, Clone)]
pub(super) struct Span {
    pub at: Loc,
    start: usize,
    end: usize,
}
struct File<'a> {
    path: PathBuf,
    text: Cow<'a, str>,
}
pub(super) struct Deck<'a> {
    files: Vec<File<'a>>,
}

struct Cursor {
    file: usize,
    start: usize,
    line: usize,
    ended: bool,
    definition: Option<Loc>,
}
impl Cursor {
    fn new(file: usize) -> Self {
        Self {
            file,
            start: 0,
            line: 0,
            ended: false,
            definition: None,
        }
    }
    fn next(&mut self, text: &str) -> Option<Span> {
        let tail = text.get(self.start..)?;
        if tail.is_empty() {
            return None;
        }
        let start = self.start;
        let end = start + tail.find('\n').unwrap_or(tail.len());
        self.start = end + usize::from(end < text.len());
        self.line += 1;
        Some(Span {
            at: Loc {
                file: self.file,
                line: self.line,
            },
            start,
            end,
        })
    }
}

impl<'a> Deck<'a> {
    pub fn text(&self, s: Span) -> &str {
        self.files[s.at.file].text[s.start..s.end].trim()
    }
    pub fn error(&self, at: Loc, message: impl Into<String>) -> FileError {
        FileError {
            path: self.files[at.file].path.clone(),
            error: ParseError::new(at.line, message),
        }
    }
    pub fn load(text: &'a str, path: Option<&Path>) -> Result<(Self, Vec<Span>), FileError> {
        let root = match path {
            Some(p) => fs::canonicalize(p).map_err(|e| FileError {
                path: p.into(),
                error: ParseError::new(1, e.to_string()),
            })?,
            None => PathBuf::from("<memory>"),
        };
        let mut index = HashMap::from([(root.clone(), 0)]);
        let mut deck = Self {
            files: vec![File {
                path: root,
                text: Cow::Borrowed(text),
            }],
        };
        let mut cards = Vec::new();
        let mut stack = vec![Cursor::new(0)];
        let mut active = HashSet::from([0]);
        while let Some(cursor) = stack.last_mut() {
            let Some(span) = cursor.next(&deck.files[cursor.file].text) else {
                if let Some(at) = cursor.definition {
                    return Err(deck.error(at, "missing .ends for subcircuit"));
                }
                active.remove(&cursor.file);
                stack.pop();
                continue;
            };
            let card = deck.text(span);
            let Some(word) = card.split_whitespace().next() else {
                continue;
            };
            if word.starts_with('*') {
                continue;
            }
            if cursor.ended {
                return Err(deck.error(span.at, "unexpected content after .end"));
            }
            if word.eq_ignore_ascii_case(".end") {
                if cursor.definition.is_some() || card.len() != word.len() {
                    return Err(deck.error(span.at, "expected .end outside a subcircuit"));
                }
                cursor.ended = true;
                continue;
            }
            if word.eq_ignore_ascii_case(".subckt") {
                if cursor.definition.replace(span.at).is_some() {
                    return Err(deck.error(span.at, "nested .subckt declaration"));
                }
            } else if word.eq_ignore_ascii_case(".ends") {
                if cursor.definition.take().is_none() {
                    return Err(deck.error(span.at, ".ends outside a subcircuit"));
                }
            } else if word.eq_ignore_ascii_case(".include") {
                if cursor.definition.is_some() {
                    return Err(deck.error(span.at, ".include must be at file scope"));
                }
                if path.is_none() {
                    return Err(deck.error(
                        span.at,
                        ".include requires a file-backed deck; use parse_file",
                    ));
                }
                let raw = card[word.len()..].trim();
                let name = if let Some(quoted) = raw.strip_prefix('"') {
                    quoted
                        .strip_suffix('"')
                        .filter(|s| !s.contains('"') && !s.is_empty())
                } else {
                    Some(raw).filter(|s| s.split_whitespace().count() == 1 && !s.contains('"'))
                };
                let name = name
                    .ok_or_else(|| deck.error(span.at, "expected one file path after .include"))?;
                let requested = deck.files[cursor.file]
                    .path
                    .parent()
                    .unwrap_or(Path::new("."))
                    .join(name);
                let canonical = fs::canonicalize(&requested).map_err(|e| {
                    deck.error(span.at, format!("include {}: {e}", requested.display()))
                })?;
                let target = if let Some(&i) = index.get(&canonical) {
                    i
                } else {
                    let contents = fs::read_to_string(&canonical).map_err(|e| {
                        deck.error(span.at, format!("include {}: {e}", canonical.display()))
                    })?;
                    let i = deck.files.len();
                    index.insert(canonical.clone(), i);
                    deck.files.push(File {
                        path: canonical,
                        text: Cow::Owned(contents),
                    });
                    i
                };
                if !active.insert(target) {
                    return Err(deck.error(
                        span.at,
                        format!("recursive include of {}", deck.files[target].path.display()),
                    ));
                }
                stack.push(Cursor::new(target));
                continue;
            }
            cards.push(span);
        }
        Ok((deck, cards))
    }
}
