//! Independent Johnson–Nyquist sources in a loaded RC ladder.
mod density;
mod stage;
pub use density::VoltageNoisePsd;
pub use stage::{NoiseError, ThermalStage};

use super::section::check_frequency;
use crate::{AcError, Complex64, Dimensionless, ElectricalImpedance, Frequency};

/// A borrowed input-to-output ladder driven by an ideal noiseless voltage source.
///
/// ```
/// use electra_ac::{Capacitance, ElectricConductance, ElectricalImpedance, Frequency, Stage,
///     ThermodynamicTemperature, noise::{NoiseLadder, ThermalStage}};
/// let resistor = Stage::new(ElectricalImpedance::from_base(1000.), Capacitance::default(),
///     ElectricConductance::default())?;
/// let stages = [ThermalStage::new(resistor, ThermodynamicTemperature::from_base(300.),
///     ThermodynamicTemperature::default())?];
/// let psd = NoiseLadder::new(&stages)?.at(Frequency::from_base(1000.))?;
/// let mean_square_voltage = psd * Frequency::from_base(100.); // 100 Hz flat bandwidth
/// assert!((mean_square_voltage.into_base() / 1.6567788e-15 - 1.).abs() < 1e-14);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub struct NoiseLadder<'a>(&'a [ThermalStage]);

impl<'a> NoiseLadder<'a> {
    /// Borrow the prepared stages without copying or allocating.
    pub fn new(stages: &'a [ThermalStage]) -> Result<Self, AcError> {
        if stages.is_empty() {
            Err(AcError::Empty)
        } else {
            Ok(Self(stages))
        }
    }

    /// Evaluate the final node's one-sided voltage PSD in V²/Hz at a frequency
    /// in Hz. At zero hertz this returns the right-hand limit of the spectrum.
    /// The stage loop uses constant storage and does not retain a sweep history.
    pub fn at(self, frequency: Frequency) -> Result<VoltageNoisePsd, AcError> {
        check_frequency(frequency)?;
        let one = Dimensionless::from_base(Complex64::from(1.));
        let mut impedance = ElectricalImpedance::default();
        let mut psd = VoltageNoisePsd::default();
        for (stage, sources) in self.0.iter().enumerate() {
            let (gain, loaded) = sources.cell.loaded(one, impedance, frequency, stage)?;
            let series = psd + sources.series_noise;
            let from_series = if series.as_base().is_finite() {
                density::propagate(series, gain)
            } else {
                // A loaded stage may attenuate two very large source densities
                // enough that their output sum is still representable.
                density::propagate(psd, gain) + density::propagate(sources.series_noise, gain)
            };
            let next = from_series + density::propagate(sources.shunt_noise, loaded);
            if !next.as_base().is_finite()
                || (next.into_base() == 0.
                    && (series.into_base() > 0. || sources.shunt_noise.into_base() > 0.))
            {
                return Err(AcError::DynamicRange { stage });
            }
            impedance = loaded;
            psd = next;
        }
        Ok(psd)
    }
}
