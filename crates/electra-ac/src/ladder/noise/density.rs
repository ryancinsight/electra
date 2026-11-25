//! Typed one-sided source densities and propagation through a complex transfer.
use crate::{Complex64, ElectricPotential, Frequency};
use aequitas::{Quantity, systems::si::quantities::ElectricCurrent};
use core::ops::{Div, Mul};

/// Mean-square voltage per unit positive-frequency bandwidth, in V²/Hz.
/// Multiplying this quantity by bandwidth produces a mean-square voltage.
pub type VoltageNoisePsd = <<ElectricPotential as Mul>::Output as Div<Frequency>>::Output;
pub(super) type CurrentNoisePsd = <<ElectricCurrent as Mul>::Output as Div<Frequency>>::Output;

/// Multiply a voltage or current source PSD by the squared magnitude of its
/// transfer. Weight each component before squaring to retain representable
/// results even when squaring the transfer alone would overflow or underflow.
#[inline]
pub(super) fn propagate<S, D, P>(source: S, transfer: Quantity<Complex64, D>) -> VoltageNoisePsd
where
    S: Copy + Mul<Quantity<f64, D>, Output = P>,
    P: Mul<Quantity<f64, D>, Output = VoltageNoisePsd>,
{
    let z = transfer.into_base();
    let re = Quantity::from_base(z.re);
    let im = Quantity::from_base(z.im);
    (source * re) * re + (source * im) * im
}
