//! Electrical analyses and projection of network results to named probes.
mod dc;
mod excitation;
mod operating_point;
pub(crate) mod sweep;
pub use operating_point::DcResult;
pub use sweep::{DcSample, DcSweep, LinearSweep, SourceValue, SweepError, SweepRange};

pub(crate) mod mna;
