//! Headless AC analysis of RC ladders, thermal noise, capacitor banks and coupled windings.
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
pub mod interconnect;
mod ladder;
pub mod magnetics;
pub mod nodal;
pub mod pdn;
mod port;
mod response;
mod summation;
mod sweep;
pub use summation::PhasorSum;

pub use aequitas::systems::si::quantities::{
    Angle, Capacitance, Dimensionless, ElectricConductance, ElectricCurrent, ElectricPotential,
    ElectricalImpedance, Frequency, Inductance, Length, MagneticPermeability, Permittivity, Power,
    ReciprocalTemperature, TemperatureDifference, ThermodynamicTemperature, Time,
};
pub use error::AcError;
pub use eunomia::Complex64;
pub use ladder::{RcLadder, Stage, noise};
pub use port::PortImpedance;
pub use response::AcPoint;
pub use sweep::LogSweep;
