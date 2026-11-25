use crate::{Angle, Complex64, Dimensionless, ElectricPotential, ElectricalImpedance, Frequency};

/// A complex voltage transfer and output impedance at one physical frequency.
#[derive(Clone, Copy, Debug)]
pub struct AcPoint {
    /// Cyclic frequency in Hz.
    pub frequency: Frequency,
    /// Output voltage divided by ideal source voltage.
    pub gain: Dimensionless<Complex64>,
    /// Driving-point impedance at the output with the input source shorted.
    pub output_impedance: ElectricalImpedance<Complex64>,
}
impl AcPoint {
    /// Linear voltage ratio.
    pub fn magnitude(&self) -> Dimensionless {
        let z = self.gain.into_base();
        Dimensionless::from_base(z.re.hypot(z.im))
    }
    /// Principal phase in radians in the interval [−π, π]. Zero gain has undefined phase.
    pub fn phase(&self) -> Option<Angle> {
        let z = self.gain.into_base();
        (z.re != 0. || z.im != 0.).then(|| Angle::from_base(z.im.atan2(z.re)))
    }
    /// Apply the transfer to a source voltage phasor using the same amplitude convention.
    pub fn output_voltage(
        &self,
        source: ElectricPotential<Complex64>,
    ) -> ElectricPotential<Complex64> {
        source * self.gain
    }
}
