use super::TwoPort;
use crate::{Complex64, Frequency, Time};
use std::f64::consts::TAU;

/// Causal reciprocal two-port fit with real modal gains and a common delay.
///
/// The even and odd gains are the eigenvalues of the symmetric scattering
/// matrix before multiplication by the all-pass delay. The model is passive
/// at every frequency exactly when both gain magnitudes are at most one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayedReciprocalModel {
    delay: f64,
    even_gain: f64,
    odd_gain: f64,
}

impl DelayedReciprocalModel {
    /// Construct a model from a nonnegative delay and finite modal gains.
    pub fn new(delay: Time, even_gain: f64, odd_gain: f64) -> Option<Self> {
        let delay = delay.into_base();
        (delay.is_finite() && delay >= 0. && even_gain.is_finite() && odd_gain.is_finite())
            .then_some(Self {
                delay,
                even_gain,
                odd_gain,
            })
    }

    /// Return one minus the largest scattering singular value.
    pub fn passivity_margin(self) -> f64 {
        1. - self.even_gain.abs().max(self.odd_gain.abs())
    }

    /// Project both orthogonal power-wave modes onto the passive unit interval.
    ///
    /// This is the nearest passive model in the modal Euclidean norm. It is not
    /// a sample-wise clipping rule: the repaired modal bound holds over the
    /// model's complete frequency domain.
    pub fn project_passive(self) -> Self {
        Self {
            delay: self.delay,
            even_gain: self.even_gain.clamp(-1., 1.),
            odd_gain: self.odd_gain.clamp(-1., 1.),
        }
    }

    /// Evaluate at a finite nonnegative frequency in hertz.
    ///
    /// Returns `None` when the input or the computed phase is unrepresentable.
    pub fn at(self, frequency: Frequency) -> Option<TwoPort> {
        let frequency = frequency.into_base();
        let angle = -TAU * (frequency * self.delay);
        if !frequency.is_finite() || frequency < 0. || !angle.is_finite() {
            return None;
        }
        let phase = Complex64::cis(angle);
        let port = TwoPort {
            reflection: phase * (self.even_gain * 0.5 + self.odd_gain * 0.5),
            transmission: phase * (self.even_gain * 0.5 - self.odd_gain * 0.5),
        };
        [port.reflection, port.transmission]
            .iter()
            .all(|value| value.re.is_finite() && value.im.is_finite())
            .then_some(port)
    }
}
