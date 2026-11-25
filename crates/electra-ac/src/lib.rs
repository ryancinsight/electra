//! Headless small-signal response of loaded RC ladders and capacitor banks.
//!
//! ```
//! use electra_ac::{Capacitance, ElectricConductance, ElectricalImpedance, Frequency, RcLadder, Stage};
//!
//! let cells = [Stage::new(ElectricalImpedance::from_base(1000.), Capacitance::from_base(1e-6),
//!                         ElectricConductance::default())?];
//! let filter = RcLadder::new(&cells)?;
//! let point = filter.at(Frequency::from_base(1000. / std::f64::consts::TAU))?;
//! assert!((point.magnitude().into_base() - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-14);
//! # Ok::<(), electra_ac::AcError>(())
//! ```
mod error;
mod ladder;
pub mod pdn;
mod response;
mod sweep;

pub use aequitas::systems::si::quantities::{
    Angle, Capacitance, Dimensionless, ElectricConductance, ElectricPotential, ElectricalImpedance,
    Frequency, Inductance,
};
pub use error::AcError;
pub use eunomia::Complex64;
pub use ladder::{RcLadder, Stage};
pub use response::AcPoint;
pub use sweep::LogSweep;
