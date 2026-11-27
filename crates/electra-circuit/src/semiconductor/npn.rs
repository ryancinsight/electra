//! Reciprocal Ebers–Moll model with ideal transport junctions.
use super::{JunctionDiode, ThermodynamicTemperature};
use crate::{Dimensionless, ElectricCurrent, ElectricPotential};

/// Ideal DC NPN at a fixed temperature, with unit junction emission factors.
/// Early effect, high injection, series resistance and stored charge are excluded.
pub struct Npn {
    junction: JunctionDiode,
    forward_beta: f64,
    reverse_beta: f64,
}

/// Conventional terminal currents, positive into the transistor.
pub struct NpnPoint {
    /// Collector current.
    pub collector: ElectricCurrent,
    /// Base current.
    pub base: ElectricCurrent,
    /// Emitter current.
    pub emitter: ElectricCurrent,
}

impl Npn {
    /// Construct from transport saturation current at the supplied temperature
    /// and positive finite forward and reverse common-emitter gains.
    pub fn new(
        saturation: ElectricCurrent,
        forward_beta: Dimensionless,
        reverse_beta: Dimensionless,
        temperature: ThermodynamicTemperature,
    ) -> Option<Self> {
        let forward_beta = forward_beta.into_base();
        let reverse_beta = reverse_beta.into_base();
        if ![forward_beta, reverse_beta]
            .iter()
            .all(|b| b.is_finite() && *b > 0.)
        {
            return None;
        }
        Some(Self {
            junction: JunctionDiode::new(saturation, Dimensionless::from_base(1.), temperature)?,
            forward_beta,
            reverse_beta,
        })
    }
    /// Evaluate terminal currents from the base-emitter and base-collector voltages.
    pub fn at(&self, vbe: ElectricPotential, vbc: ElectricPotential) -> Option<NpnPoint> {
        let forward = self.junction.current(vbe)?;
        let reverse = self.junction.current(vbc)?;
        let transport = forward - reverse;
        let base_f = forward / self.forward_beta;
        let base_r = reverse / self.reverse_beta;
        let collector = transport - base_r;
        let base = base_f + base_r;
        let emitter = -transport - base_f;
        [collector, base, emitter]
            .iter()
            .all(|i| i.is_finite())
            .then_some(NpnPoint {
                collector: ElectricCurrent::from_base(collector),
                base: ElectricCurrent::from_base(base),
                emitter: ElectricCurrent::from_base(emitter),
            })
    }
}
