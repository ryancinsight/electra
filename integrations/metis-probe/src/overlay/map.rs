use super::Overlay;
use electra_ac::*;
use electra_board::{
    board::{LayerId, NetId, Track},
    geom::{Nm, Point},
};
use electra_board_field::rail::{Resistivity, ThermalResistance, solve};

/// Row-major cells with one uniform trace and net per cell.
pub struct RailMap {
    shape: [usize; 2],
    origin: [Length; 2],
    pitch: Length,
    nets: Vec<NetId>,
    volts: Vec<ElectricPotential>,
    rise: Vec<TemperatureDifference>,
}
impl RailMap {
    /// Parse and solve the whitespace-delimited board rail format in docs/field/ports.md.
    pub fn parse(input: &str) -> Option<Self> {
        let mut rows = input
            .lines()
            .filter(|s| !s.trim().is_empty() && !s.trim_start().starts_with('#'));
        let [h, w, x, y, pitch] = columns(rows.next()?)?;
        let mut map = Self {
            shape: [h.parse().ok()?, w.parse().ok()?],
            origin: [
                Length::from_base(number(x)? * 1e-3),
                Length::from_base(number(y)? * 1e-3),
            ],
            pitch: Length::from_base(number(pitch)? * 1e-3),
            nets: vec![],
            volts: vec![],
            rise: vec![],
        };
        for row in rows {
            let [net, dx, dy, width, thickness, rho, alpha, theta, current] = columns(row)?;
            let track = Track {
                start: Point::default(),
                end: Point::new(Nm::from_mm(number(dx)?), Nm::from_mm(number(dy)?)),
                width: Nm::from_mm(number(width)?),
                layer: LayerId(0),
                net: NetId(net.parse().ok()?),
            };
            let sample = solve(
                &track,
                Length::from_base(number(thickness)? * 1e-6),
                Resistivity::from_base(number(rho)?),
                ReciprocalTemperature::from_base(number(alpha)?),
                ThermalResistance::from_base(number(theta)?),
                ElectricCurrent::from_base(number(current)?),
            )?;
            map.nets.push(track.net);
            map.volts.push(sample.drop);
            map.rise.push(sample.rise);
        }
        if map.shape.contains(&0)
            || map.shape[0].checked_mul(map.shape[1])? != map.nets.len()
            || map.pitch.into_base() <= 0.
        {
            return None;
        }
        Some(map)
    }
    /// Borrow both fields for drawing or cursor inspection.
    pub fn view(&self) -> Overlay<'_> {
        Overlay::new(
            self.shape,
            self.origin,
            self.pitch,
            &self.nets,
            &self.volts,
            &self.rise,
        )
        .expect("validated rail map")
    }
    /// Grid dimensions in rows and columns.
    pub fn shape(&self) -> [usize; 2] {
        self.shape
    }
}
fn columns<const N: usize>(line: &str) -> Option<[&str; N]> {
    let mut words = line.split_whitespace();
    let result = std::array::from_fn(|_| words.next().unwrap_or_default());
    (words.next().is_none() && !result.contains(&"")).then_some(result)
}
fn number(token: &str) -> Option<f64> {
    let n: f64 = token.parse().ok()?;
    n.is_finite().then_some(n)
}
