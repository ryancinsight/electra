//! Long-channel square-law NMOS with bulk tied to source.
use crate::{ElectricConductance, ElectricCurrent, ElectricPotential};

/// Static level-1 channel with zero channel-length modulation and zero body effect.
pub struct LongChannelNmos {
    threshold: ElectricPotential,
    beta: f64,
}
/// Drain current and small-signal gate and drain derivatives.
pub struct NmosPoint {
    /// Conventional current into the drain.
    pub drain: ElectricCurrent,
    /// Gate transconductance at fixed drain voltage.
    pub gm: ElectricConductance,
    /// Drain conductance at fixed gate voltage.
    pub gds: ElectricConductance,
}
impl LongChannelNmos {
    /// Set beta (KP × W/L) from saturation transconductance at a positive reference overdrive.
    /// Threshold and gain are operating-temperature parameters.
    pub fn new(
        threshold: ElectricPotential,
        reference_overdrive: ElectricPotential,
        reference_gm: ElectricConductance,
    ) -> Option<Self> {
        let overdrive = reference_overdrive.into_base();
        let gm = reference_gm.into_base();
        if !threshold.into_base().is_finite()
            || !overdrive.is_finite()
            || overdrive <= 0.
            || !gm.is_finite()
            || gm <= 0.
        {
            return None;
        }
        let beta = gm / overdrive;
        (beta.is_finite() && beta > 0.).then_some(Self { threshold, beta })
    }
    /// Evaluate the forward channel for nonnegative Vds. Cutoff returns zero current and slopes.
    pub fn at(&self, vgs: ElectricPotential, vds: ElectricPotential) -> Option<NmosPoint> {
        let drain = vds.into_base();
        let gate = vgs.into_base();
        if !gate.is_finite() || !drain.is_finite() || drain < 0. {
            return None;
        }
        let overdrive = (gate - self.threshold.into_base()).max(0.);
        let channel = overdrive.min(drain);
        let gm = self.beta * channel;
        let gds = self.beta * (overdrive - channel);
        let current = gm * (overdrive - 0.5 * channel);
        [gm, gds, current]
            .iter()
            .all(|v| v.is_finite())
            .then_some(NmosPoint {
                drain: ElectricCurrent::from_base(current),
                gm: ElectricConductance::from_base(gm),
                gds: ElectricConductance::from_base(gds),
            })
    }
}
