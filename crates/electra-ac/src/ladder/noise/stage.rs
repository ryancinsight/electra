//! Prepare independent series-voltage and shunt-current noise sources once.
use super::density::{CurrentNoisePsd, VoltageNoisePsd};
use crate::{Stage, ThermodynamicTemperature};
use aequitas::systems::si::constants::BOLTZMANN_CONSTANT;

/// A physical stage and its classical, one-sided resistor noise densities.
#[derive(Clone, Copy, Debug)]
pub struct ThermalStage {
    pub(super) cell: Stage,
    pub(super) series_noise: VoltageNoisePsd,
    pub(super) shunt_noise: CurrentNoisePsd,
}

/// Invalid temperature or a source density outside the scalar range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NoiseError {
    /// Temperatures are absolute, nonnegative kelvin.
    Temperature,
    /// A positive source density could not be represented as a finite nonzero value.
    SourceRange,
}
impl core::fmt::Display for NoiseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Temperature => "resistor temperature must be finite and nonnegative in kelvin",
            Self::SourceRange => "thermal source density exceeds the numeric range",
        })
    }
}
impl std::error::Error for NoiseError {}

impl ThermalStage {
    /// Prepare independent noise for the series resistor and shunt conductance.
    /// Zero kelvin disables that source in the classical model.
    pub fn new(
        cell: Stage,
        series_t: ThermodynamicTemperature,
        shunt_t: ThermodynamicTemperature,
    ) -> Result<Self, NoiseError> {
        if [series_t, shunt_t]
            .into_iter()
            .any(|t| !t.as_base().is_finite() || t.into_base() < 0.)
        {
            return Err(NoiseError::Temperature);
        }
        let k4 = BOLTZMANN_CONSTANT * 4.;
        // Multiply the larger input by 4k first to avoid premature underflow at very low T.
        let series_noise: VoltageNoisePsd = if series_t.into_base() >= cell.series.into_base() {
            (k4 * series_t) * cell.series
        } else {
            (k4 * cell.series) * series_t
        };
        let shunt_noise: CurrentNoisePsd = if shunt_t.into_base() >= cell.load.into_base() {
            (k4 * shunt_t) * cell.load
        } else {
            (k4 * cell.load) * shunt_t
        };
        if !series_noise.as_base().is_finite()
            || !shunt_noise.as_base().is_finite()
            || (series_t.into_base() > 0. && series_noise.into_base() == 0.)
            || (shunt_t.into_base() > 0.
                && cell.load.into_base() != 0.
                && shunt_noise.into_base() == 0.)
        {
            return Err(NoiseError::SourceRange);
        }
        Ok(Self {
            cell,
            series_noise,
            shunt_noise,
        })
    }
}
