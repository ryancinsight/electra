//! Independent field and distributed-line references.
use electra_ac::{interconnect::*, *};
use std::f64::consts::{PI, TAU};

#[test]
fn annular_flux_converges_to_gauss_and_ampere_solutions() {
    let (a, b, eps, mu) = (0.00025_f64, 0.001, 2.1 * 8.8541878128e-12, 4e-7 * PI);
    let log = (b / a).ln();
    let exact = [TAU * eps / log, mu * log / TAU];
    let mut previous = exact;
    for shells in [8, 16, 32] {
        let section = CoaxSection::extract(
            Length::from_base(a),
            Length::from_base(b),
            Permittivity::from_base(eps),
            MagneticPermeability::from_base(mu),
            shells,
        )
        .unwrap();
        let actual = [
            section.capacitance_per_length.into_base(),
            section.inductance_per_length.into_base(),
        ];
        for i in 0..2 {
            assert!((actual[i] / exact[i] - 1.).abs() < 0.02);
            if shells == 32 {
                assert!((actual[i] / previous[i] - 1.).abs() < 0.01);
            }
        }
        previous = actual;
    }
    assert!(
        CoaxSection::extract(
            Length::from_base(b),
            Length::from_base(a),
            Permittivity::from_base(eps),
            MagneticPermeability::from_base(mu),
            8
        )
        .is_none()
    );
}

#[test]
fn reflected_load_and_phase_slope_for_one_metre_line() {
    let line = Line::new(
        ElectricalImpedance::default(),
        Inductance::from_base(250e-9),
        ElectricConductance::default(),
        Capacitance::from_base(100e-12),
    )
    .unwrap();
    let z0 = ElectricalImpedance::from_base(50.);
    let expected_delay = 5e-9;
    let mut previous: Option<Complex64> = None;
    for mhz in 1..=200 {
        let f = f64::from(mhz) * 1e6;
        let s = line.at(Frequency::from_base(f), z0).unwrap();
        // Independent travelling-wave phase and boundary conditions, both directions.
        for load in [-1., 0., 1.] {
            let normalized = s.terminated(Complex64::from(load)).unwrap()
                * Complex64::cis(2. * TAU * f * expected_delay);
            assert!((normalized - load).norm() < 0.01);
        }
        assert!((s.transmission - Complex64::cis(-TAU * f * expected_delay)).norm() < 1e-10);
        assert!((s.maximum_gain() - 1.).abs() < 1e-12);
        if let Some(last) = previous {
            let delay = -(s.transmission / last).arg() / (TAU * 1e6);
            assert!((delay / expected_delay - 1.).abs() < 0.01);
        }
        previous = Some(s.transmission);
    }
    assert!((line.flight_time().into_base() / expected_delay - 1.).abs() < 1e-12);
}

#[test]
fn lossy_line_matches_independent_chain_matrix() {
    let line = Line::new(
        ElectricalImpedance::from_base(3.),
        Inductance::from_base(250e-9),
        ElectricConductance::from_base(0.001),
        Capacitance::from_base(100e-12),
    )
    .unwrap();
    for mhz in 1..=200 {
        let w = TAU * f64::from(mhz) * 1e6;
        let s = line
            .at(
                Frequency::from_base(w / TAU),
                ElectricalImpedance::from_base(75.),
            )
            .unwrap();
        let z = Complex64::new(3., w * 250e-9);
        let y = Complex64::new(0.001, w * 100e-12);
        let g = (z * y).sqrt();
        let forward = g.exp();
        let backward = (-g).exp();
        let a = (forward + backward) * 0.5;
        let sinh = (forward - backward) * 0.5;
        let b = z / g * sinh;
        let c = y / g * sinh;
        let den = a * 2. + b / 75. + c * 75.;
        assert!((s.transmission - Complex64::from(2.) / den).norm() < 1e-10);
        assert!((s.transmission - (a * a - b * c) * 2. / den).norm() < 1e-10);
        assert!((s.reflection - (b / 75. - c * 75.) / den).norm() < 1e-10);
        assert!(s.maximum_gain() <= 1. + 1e-12);
    }
}
