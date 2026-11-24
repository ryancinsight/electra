//! Electrical analyses and projection of network results to named probes.
mod operating_point;
pub(crate) mod sweep;
pub use operating_point::DcResult;
pub use sweep::{DcSample, DcSweep, LinearSweep, SourceValue, SweepError, SweepRange};
