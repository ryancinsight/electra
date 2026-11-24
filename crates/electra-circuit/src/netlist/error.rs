use std::{fmt, path::PathBuf};

/// A line-numbered netlist error.
#[derive(Debug, PartialEq)]
pub struct ParseError {
    /// One-based line in the originating deck.
    pub line: usize,
    /// Invalid card, value or connection.
    pub message: String,
}
impl ParseError {
    pub(super) fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}
impl std::error::Error for ParseError {}

/// A netlist diagnostic located in a root or included file.
#[derive(Debug)]
pub struct FileError {
    /// File containing the failing card, or the requested path for a root read failure.
    pub path: PathBuf,
    /// Line and explanation.
    pub error: ParseError,
}
impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: {}",
            self.path.display(),
            self.error.line,
            self.error.message
        )
    }
}
impl std::error::Error for FileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}
