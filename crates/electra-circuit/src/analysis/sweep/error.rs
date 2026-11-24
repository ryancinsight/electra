use super::SourceValue;
use crate::SolveError;
use std::fmt;

/// Failure to specify a DC sweep or compute one of its samples.
#[derive(Debug)]
pub enum SweepError {
    /// The deck does not define a .dc card.
    MissingDirective,
    /// Element lookup failed.
    UnknownSource(String),
    /// The element is passive or the requested range has the wrong dimension.
    SourceType(String),
    /// The interval is invalid or adjacent samples cannot be distinguished.
    InvalidRange(&'static str),
    /// Topology preparation failed.
    Network(SolveError),
    /// A point failed its operating-point checks.
    Point(SourceValue, SolveError),
}
impl fmt::Display for SweepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDirective => f.write_str("deck has no .dc directive"),
            Self::UnknownSource(s) => write!(f, "unknown sweep source {s}"),
            Self::SourceType(s) => write!(
                f,
                "sweep requires an independent source with matching units: {s}"
            ),
            Self::InvalidRange(s) => f.write_str(s),
            Self::Network(e) => e.fmt(f),
            Self::Point(q, e) => write!(f, "sweep failed at {q}: {e}"),
        }
    }
}
impl std::error::Error for SweepError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Network(e) | Self::Point(_, e) => Some(e),
            _ => None,
        }
    }
}
