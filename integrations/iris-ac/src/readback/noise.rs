use electra_ac::noise::VoltageNoisePsd;

/// One-sided voltage amplitude spectral density in V/√Hz.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoltageNoiseAsd(f64);

impl VoltageNoiseAsd {
    /// Take the square root of a finite, nonnegative one-sided voltage PSD.
    pub fn from_psd(density: VoltageNoisePsd) -> Option<Self> {
        let density = density.into_base();
        (density.is_finite() && density >= 0.).then(|| Self(density.sqrt()))
    }

    /// Numeric amplitude spectral density in V/√Hz.
    pub fn volts_per_root_hertz(self) -> f64 {
        self.0
    }
}
