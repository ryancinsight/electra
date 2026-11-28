//! Lumped steady-state copper loss with a prescribed constant current.
use electra_ac::{
    ElectricCurrent, ElectricPotential, ElectricalImpedance, Length, Power, ReciprocalTemperature,
    TemperatureDifference,
};
use electra_board::board::Track;

/// Material resistivity in Ω m, at the ambient reference temperature.
pub type Resistivity = <ElectricalImpedance as std::ops::Mul<Length>>::Output;
/// Effective path resistance to ambient in K/W.
pub type ThermalResistance = <TemperatureDifference as std::ops::Div<Power>>::Output;

/// Steady-state result for one uniform copper segment.
#[derive(Debug, Clone, Copy)]
pub struct RailSample {
    /// Signed voltage loss along the selected current orientation.
    pub drop: ElectricPotential,
    /// Dissipation in the hot conductor.
    pub power: Power,
    /// Temperature above the ambient reference.
    pub rise: TemperatureDifference,
}

/// Solve R(T)=ρl/(wt)·(1+αΔT) and ΔT=θI²R(T) at constant current.
/// Returns None for invalid geometry, negative loss parameters, or runaway.
pub fn solve(
    track: &Track,
    thickness: Length,
    resistivity: Resistivity,
    alpha: ReciprocalTemperature,
    theta: ThermalResistance,
    current: ElectricCurrent,
) -> Option<RailSample> {
    let length = track.start.euclid(track.end) * 1e-9;
    let width = track.width.to_mm() * 1e-3;
    let (t, rho, alpha, theta, i) = (
        thickness.into_base(),
        resistivity.into_base(),
        alpha.into_base(),
        theta.into_base(),
        current.into_base(),
    );
    if ![length, width, t, rho]
        .iter()
        .all(|x| x.is_finite() && *x > 0.)
        || ![alpha, theta].iter().all(|x| x.is_finite() && *x >= 0.)
        || !i.is_finite()
    {
        return None;
    }
    let cold = rho * length / (width * t);
    let loop_gain = theta * (i * i * cold) * alpha;
    if loop_gain >= 1. {
        return None;
    }
    let drop = i * cold / (1. - loop_gain);
    let power = i * drop;
    let rise = theta * power;
    [drop, power, rise]
        .iter()
        .all(|x| x.is_finite())
        .then(|| RailSample {
            drop: ElectricPotential::from_base(drop),
            power: Power::from_base(power),
            rise: TemperatureDifference::from_base(rise),
        })
}
