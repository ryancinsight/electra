//! Linear network primitives shared by electrical and hydraulic adapters.
//! Endpoints are dense zero-based node indices. Each connected component
//! requires a prescribed potential; positive injection enters a node.
mod branch;
mod relaxation;
mod solve;
pub use branch::Branch;
pub use relaxation::{RelaxationRow, sweep};
pub use solve::{Error, OperatingPoint, SolveOptions, solve};
