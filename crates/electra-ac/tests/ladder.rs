//! Closed-form circuit references and model-domain regressions.
use aequitas::systems::si::units::Degree;
use electra_ac::{
    AcError, Capacitance, Complex64, ElectricConductance, ElectricPotential, ElectricalImpedance,
    Frequency, RcLadder, Stage,
};
use std::f64::consts::{FRAC_1_SQRT_2, TAU};

fn stage(r: f64, c: f64, g: f64) -> Stage {
    Stage::new(
        ElectricalImpedance::from_base(r),
        Capacitance::from_base(c),
        ElectricConductance::from_base(g),
    )
    .unwrap()
}
fn near(got: f64, want: f64, tol: f64) {
    assert!((got - want).abs() <= tol, "{got} vs {want}");
}

#[test]
fn one_pole_checks_real_imag_phase_and_voltage() {
    let stages = [stage(1000., 1e-6, 0.)];
    let model = RcLadder::new(&stages).unwrap();
    for x in [0.1_f64, 1., 10.] {
        let p = model.at(Frequency::from_base(x / (TAU * 1e-3))).unwrap();
        let z = p.gain.into_base();
        near(z.re, 1. / (1. + x * x), 1e-14);
        near(z.im, -x / (1. + x * x), 1e-14);
        near(p.magnitude().into_base(), 1. / (1. + x * x).sqrt(), 1e-14);
        near(
            p.phase().unwrap().in_unit::<Degree>(),
            -x.atan().to_degrees(),
            1e-11,
        );
        if x == 1. {
            near(p.magnitude().into_base(), FRAC_1_SQRT_2, 1e-14);
            let v = p
                .output_voltage(ElectricPotential::from_base(Complex64::new(0., 2.)))
                .into_base();
            near(v.re, 1., 1e-13);
            near(v.im, 1., 1e-13);
        }
    }
}
#[test]
fn identical_unbuffered_sections_load_each_other() {
    // For two identical sections H(s) = 1/(1 + 3sRC + (sRC)^2).
    let stages = [stage(1000., 1e-6, 0.); 2];
    let p = RcLadder::new(&stages)
        .unwrap()
        .at(Frequency::from_base(1000. / TAU))
        .unwrap();
    near(p.gain.into_base().re, 0., 1e-14);
    near(p.gain.into_base().im, -1. / 3., 1e-14);
    near(p.output_impedance.into_base().re, 1000. / 3., 1e-10);
    near(p.output_impedance.into_base().im, -2000. / 3., 1e-10);
}
#[test]
fn loaded_dc_matches_resistive_network_and_source_off_impedance() {
    let stages = [stage(2., 0.5, 0.), stage(3., 1., 0.2)];
    let p = RcLadder::new(&stages)
        .unwrap()
        .at(Frequency::from_base(0.))
        .unwrap();
    assert_eq!(p.gain.into_base(), Complex64::from(0.5));
    assert_eq!(p.output_impedance.into_base(), Complex64::from(2.5));
}
#[test]
fn large_reactance_ratio_keeps_representable_quadratures() {
    let stages = [stage(1e200, 1., 0.)];
    let p = RcLadder::new(&stages)
        .unwrap()
        .at(Frequency::from_base(1. / TAU))
        .unwrap();
    near(p.gain.into_base().im / -1e-200, 1., 1e-14);
    near(p.output_impedance.into_base().re / 1e-200, 1., 1e-14);
    near(p.output_impedance.into_base().im, -1., 1e-14);
}
#[test]
fn invalid_components_and_frequency_are_diagnosed() {
    let new = |r, c, g| {
        Stage::new(
            ElectricalImpedance::from_base(r),
            Capacitance::from_base(c),
            ElectricConductance::from_base(g),
        )
    };
    for bad in [f64::NAN, f64::INFINITY, -1.] {
        assert_eq!(new(bad, 0., 0.).unwrap_err(), AcError::Resistance);
        assert_eq!(new(1., bad, 0.).unwrap_err(), AcError::Capacitance);
        assert_eq!(new(1., 0., bad).unwrap_err(), AcError::Conductance);
    }
    assert_eq!(new(0., 0., 0.).unwrap_err(), AcError::Resistance);
    assert_eq!(RcLadder::new(&[]).unwrap_err(), AcError::Empty);
    let cells = [stage(1., 1., 0.)];
    let model = RcLadder::new(&cells).unwrap();
    for f in [f64::NAN, -1., f64::INFINITY] {
        assert_eq!(
            model.at(Frequency::from_base(f)).unwrap_err(),
            AcError::Frequency
        );
    }
    assert_eq!(
        model.at(Frequency::from_base(f64::MAX)).unwrap_err(),
        AcError::DynamicRange { stage: 0 }
    );
}
