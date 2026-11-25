//! Display and export conversions for an individual headless sample.
//! Zero amplitude maps to negative infinity on logarithmic axes.
//! Negative or non-finite inputs have no display value.
//!
//! ```compile_fail,E0308
//! use electra_ac::noise::VoltageNoisePsd;
//! use electra_iris_ac::readback::VoltageNoiseAsd;
//! fn psd_axis(_: VoltageNoisePsd) {}
//! fn wrong(asd: VoltageNoiseAsd) { psd_axis(asd); }
//! ```
//!
//! ```compile_fail,E0308
//! use electra_iris_ac::readback::{GainDb, VoltageDbv};
//! fn gain_axis(_: GainDb) {}
//! fn wrong(level: VoltageDbv) { gain_axis(level); }
//! ```
mod decibels;
mod noise;
pub use decibels::{GainDb, VoltageDbv};
pub use noise::VoltageNoiseAsd;
