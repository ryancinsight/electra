use super::CoaxSection;
use crate::{
    Capacitance, Complex64, ElectricConductance, ElectricalImpedance, Frequency, Inductance,
    Length, Time,
};
use std::f64::consts::TAU;

/// Uniform line described by its four integrated RLGC parameters over the route.
#[derive(Clone, Copy, Debug)]
pub struct Line {
    r: f64,
    l: f64,
    g: f64,
    c: f64,
}
/// Power-wave scattering coefficients for a symmetric reciprocal network.
/// The diagonal entries are equal and the off-diagonal entries are equal.
#[derive(Clone, Copy, Debug)]
pub struct TwoPort {
    /// Input (also output) reflection coefficient.
    pub reflection: Complex64,
    /// Forward (also reverse) transmission coefficient.
    pub transmission: Complex64,
}
impl TwoPort {
    /// Largest singular value of this symmetric scattering matrix.
    pub fn maximum_gain(self) -> f64 {
        (self.reflection + self.transmission)
            .norm()
            .max((self.reflection - self.transmission).norm())
    }
    /// Input reflection for a load with the supplied power-wave reflection coefficient.
    /// An undefined terminated pole or nonfinite result returns None.
    pub fn terminated(self, load: Complex64) -> Option<Complex64> {
        let input = self.reflection
            + self.transmission * self.transmission * load
                / (Complex64::from(1.) - self.reflection * load);
        (input.re.is_finite() && input.im.is_finite()).then_some(input)
    }
}
impl Line {
    /// Assemble a distributed line from the total series R/L and shunt G/C.
    pub fn new(
        r: ElectricalImpedance,
        l: Inductance,
        g: ElectricConductance,
        c: Capacitance,
    ) -> Option<Self> {
        let (r, l, g, c) = (r.into_base(), l.into_base(), g.into_base(), c.into_base());
        ([r, l, g, c].iter().all(|x| x.is_finite()) && r >= 0. && g >= 0. && l > 0. && c > 0.)
            .then_some(Self { r, l, g, c })
    }
    /// Assemble a lossless route from a solved cross section and its physical length.
    pub fn from_section(section: CoaxSection, length: Length) -> Option<Self> {
        Self::new(
            ElectricalImpedance::default(),
            Inductance::from_base(section.inductance_per_length.into_base() * length.into_base()),
            ElectricConductance::default(),
            Capacitance::from_base(section.capacitance_per_length.into_base() * length.into_base()),
        )
    }
    /// High-frequency one-way propagation time in seconds.
    pub fn flight_time(self) -> Time {
        Time::from_base(self.l.sqrt() * self.c.sqrt())
    }
    /// Solve the telegrapher equations at positive frequency with equal real port references.
    /// Uses the decaying travelling wave so long attenuating routes do not overflow cosh.
    pub fn at(self, frequency: Frequency, reference: ElectricalImpedance) -> Option<TwoPort> {
        let w = TAU * frequency.into_base();
        let reference = reference.into_base();
        if !w.is_finite() || w <= 0. || !reference.is_finite() || reference <= 0. {
            return None;
        }
        let z = Complex64::new(self.r, w * self.l);
        let y = Complex64::new(self.g, w * self.c);
        let zc = (z / y).sqrt();
        // Select the forward branch using Z = gamma * Zc, including the lossless axis.
        let gamma = z / zc;
        let propagation = Complex64::from_polar((-gamma.re).exp(), -gamma.im);
        let rho = (zc - reference) / (zc + reference);
        let one = Complex64::from(1.);
        let p2 = propagation * propagation;
        let rho2 = rho * rho;
        let denominator = one - rho2 * p2;
        let s = TwoPort {
            reflection: rho * (one - p2) / denominator,
            transmission: (one - rho2) * propagation / denominator,
        };
        [s.reflection, s.transmission]
            .iter()
            .all(|x| x.re.is_finite() && x.im.is_finite())
            .then_some(s)
    }
}
