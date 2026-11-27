//! Independent uniform resistor tolerances for an unloaded two-resistor divider.
use crate::{ElectricPotential, ElectricalImpedance};
mod uniform;
use uniform::Uniform;

/// Closed resistance interval in ohms. Equal endpoints model a fixed resistor.
#[derive(Clone, Copy)]
pub struct ResistorRange {
    low: f64,
    high: f64,
}

impl ResistorRange {
    /// Return a range for positive, finite, ordered endpoints.
    pub fn new(low: ElectricalImpedance, high: ElectricalImpedance) -> Option<Self> {
        let (low, high) = (low.into_base(), high.into_base());
        (low > 0. && high.is_finite() && low <= high).then_some(Self { low, high })
    }

    fn draw(self, rng: &mut Uniform) -> f64 {
        (self.high - self.low).mul_add(rng.next(), self.low)
    }
}

/// Divider between a finite source voltage and ground; output is across the lower resistor.
pub struct DividerTolerance {
    drive: ElectricPotential,
    upper: ResistorRange,
    lower: ResistorRange,
}

impl DividerTolerance {
    /// Bind independent upper and lower resistance ranges to a source voltage.
    pub fn new(
        drive: ElectricPotential,
        upper: ResistorRange,
        lower: ResistorRange,
    ) -> Option<Self> {
        drive.into_base().is_finite().then_some(Self {
            drive,
            upper,
            lower,
        })
    }

    fn output(&self, upper: f64, lower: f64) -> ElectricPotential {
        let scale = upper.max(lower);
        let ratio = lower / scale;
        let drive = self.drive.into_base();
        // When the divider ratio underflows, a large source can still produce a
        // representable output. Transfer its binary exponent to the numerator.
        let output = if ratio < f64::MIN_POSITIVE && drive.abs() > 1. {
            let exponent = ((drive.abs().to_bits() >> 52) & 0x7ff) << 52;
            let power = f64::from_bits(exponent);
            // In this regime upper dominates the denominator to f64 precision.
            if lower <= f64::MAX / power {
                (drive / power) * ((lower * power) / upper)
            } else {
                (drive / upper) * lower
            }
        } else {
            drive * (ratio / (upper / scale + ratio))
        };
        ElectricPotential::from_base(output)
    }

    /// Evaluate (low,low), (low,high), (high,low), (high,high) in upper/lower order.
    pub fn corners(&self) -> [ElectricPotential; 4] {
        let a = self.upper;
        let b = self.lower;
        [
            (a.low, b.low),
            (a.low, b.high),
            (a.high, b.low),
            (a.high, b.high),
        ]
        .map(|(u, l)| self.output(u, l))
    }

    /// Stream independent uniform draws in upper/lower order. The seed initializes
    /// SplitMix64; its high 53 bits map each draw into [0,1). Storage is constant.
    pub fn samples(
        &self,
        seed: u64,
        count: usize,
    ) -> impl ExactSizeIterator<Item = ElectricPotential> + '_ {
        let mut rng = Uniform(seed);
        (0..count).map(move |_| self.output(self.upper.draw(&mut rng), self.lower.draw(&mut rng)))
    }
}
