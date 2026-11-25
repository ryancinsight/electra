use electra_ac::{Dimensionless, ElectricPotential};

fn amplitude_db(ratio: f64) -> Option<f64> {
    (ratio.is_finite() && ratio >= 0.).then(|| 20. * ratio.log10())
}

/// Voltage gain in decibels, relative to a unit amplitude ratio.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GainDb(f64);

impl GainDb {
    /// Convert a nonnegative linear voltage gain.
    pub fn from_magnitude(gain: Dimensionless) -> Option<Self> {
        amplitude_db(gain.into_base()).map(Self)
    }

    /// Numeric gain in dB.
    pub fn decibels(self) -> f64 {
        self.0
    }
}

/// Absolute RMS voltage level in dBV, relative to one volt RMS.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoltageDbv(f64);

impl VoltageDbv {
    /// Convert a nonnegative RMS voltage in SI base units.
    pub fn from_rms(voltage: ElectricPotential) -> Option<Self> {
        amplitude_db(voltage.into_base()).map(Self)
    }

    /// Numeric level in dBV.
    pub fn decibels_relative_to_one_volt(self) -> f64 {
        self.0
    }
}
