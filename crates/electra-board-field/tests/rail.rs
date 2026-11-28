//! Independent iterative copper heat-balance checks.
use electra_ac::*;
use electra_board::{
    board::{LayerId, NetId, Track},
    geom::{Nm, Point},
};
use electra_board_field::rail::{Resistivity, ThermalResistance, solve};
#[test]
fn compare_hot_rail_with_iterative_heat_balance_and_reject_runaway() {
    let track = Track {
        start: Point::default(),
        end: Point::new(Nm::from_mm(60.), Nm::from_mm(80.)),
        width: Nm::from_mm(0.5),
        layer: LayerId(0),
        net: NetId(7),
    };
    for amps in [-2., 0., 2.] {
        let eval = |theta| {
            solve(
                &track,
                Length::from_base(35e-6),
                Resistivity::from_base(1.724e-8),
                ReciprocalTemperature::from_base(0.00393),
                ThermalResistance::from_base(theta),
                ElectricCurrent::from_base(amps),
            )
        };
        let hot = eval(60.).unwrap();
        let cold = 1.724e-8 * 0.1 / (0.0005 * 35e-6);
        let mut rise = 0.;
        for _ in 0..100 {
            rise = 60. * amps * amps * cold * (1. + 0.00393 * rise);
        }
        let drop = amps * cold * (1. + 0.00393 * rise);
        assert!((hot.rise.into_base() - rise).abs() < 1e-10);
        assert!((hot.drop.into_base() - drop).abs() < 1e-10);
        assert!((hot.power.into_base() - amps * drop).abs() < 1e-10);
        if amps != 0. {
            assert!(eval(1000.).is_none());
        }
    }
}
