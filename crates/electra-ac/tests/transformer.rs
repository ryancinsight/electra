//! Winding conventions, the ideal limit, termination states and representable extremes.
use electra_ac::{
    Complex64, Dimensionless, ElectricalImpedance, Frequency, Inductance, PortImpedance,
    magnetics::{
        CoupledInductors, IdealTransformer, MagneticError, SecondaryLoad, TransformerPoint,
    },
};
use std::f64::consts::TAU;

fn load(r: f64) -> SecondaryLoad {
    SecondaryLoad::Resistance(ElectricalImpedance::from_base(r))
}
fn pair(lp: f64, ls: f64, k: f64, load: SecondaryLoad) -> Result<CoupledInductors, MagneticError> {
    CoupledInductors::new(
        Inductance::from_base(lp),
        Inductance::from_base(ls),
        Dimensionless::from_base(k),
        load,
    )
}
fn ideal(n: f64, load: SecondaryLoad) -> Result<IdealTransformer, MagneticError> {
    IdealTransformer::new(Dimensionless::from_base(n), load)
}
fn impedance(point: TransformerPoint) -> Complex64 {
    match point.input_impedance {
        PortImpedance::Finite(z) => z.into_base(),
        PortImpedance::Open => panic!("expected finite primary impedance"),
    }
}
fn relative(got: f64, want: f64) {
    assert!((got / want - 1.).abs() < 2e-13, "{got:e} vs {want:e}");
}

#[test]
fn perfect_coupling_and_large_magnetizing_reactance_are_separate_limits() {
    let hz = Frequency::from_base(1. / TAU);
    let target = ideal(-2., load(40.)).unwrap().at(hz).unwrap();
    assert_eq!(impedance(target), Complex64::from(10.));
    let exact_k = pair(10., 40., -1., load(40.)).unwrap().at(hz).unwrap();
    assert_eq!(exact_k.gain, target.gain);
    // A 10-ohm reflected load in parallel with j10 ohms.
    relative(impedance(exact_k).re, 5.);
    relative(impedance(exact_k).im, 5.);
    let mut last = 1.;
    for scale in [10.0_f64, 100., 1000., 10000.] {
        // Xs/R grows while leakage reactance / R falls.
        let k = -(1. - 1. / scale.powi(2)).sqrt();
        let p = pair(10. * scale, 40. * scale, k, load(40.))
            .unwrap()
            .at(hz)
            .unwrap();
        let z = impedance(p);
        let h = p.gain.into_base();
        let err = ((z.re - 10.).hypot(z.im) / 10.).max((h.re + 2.).hypot(h.im) / 2.);
        assert!(err <= 2.001 / scale && err < last / 4.);
        last = err;
    }
}

#[test]
fn dot_reversal_conserves_power_and_negates_secondary_voltage() {
    for f in [2., 100., 1e5] {
        let hz = Frequency::from_base(f);
        let p = pair(0.01, 0.04, 0.87, load(50.)).unwrap().at(hz).unwrap();
        let n = pair(0.01, 0.04, -0.87, load(50.)).unwrap().at(hz).unwrap();
        assert_eq!(impedance(n), impedance(p));
        assert_eq!(n.gain.into_base(), -p.gain.into_base());
        let z = impedance(p);
        let h = p.gain.into_base();
        relative(
            z.re / z.re.hypot(z.im).powi(2),
            h.re.hypot(h.im).powi(2) / 50.,
        );
    }
}

#[test]
fn open_short_and_zero_coupling_have_distinct_port_laws() {
    let hz = Frequency::from_base(1. / TAU);
    let open = pair(4., 1., -0.5, SecondaryLoad::Open)
        .unwrap()
        .at(hz)
        .unwrap();
    relative(open.gain.into_base().re, -0.25);
    relative(impedance(open).im, 4.);
    assert_eq!(impedance(open).re, 0.);
    let short = pair(4., 1., -0.5, load(0.)).unwrap().at(hz).unwrap();
    assert_eq!(short.gain.into_base(), Complex64::default());
    relative(impedance(short).im, 3.);
    for r in [SecondaryLoad::Open, load(0.), load(7.)] {
        let isolated = pair(4., 1., 0., r).unwrap().at(hz).unwrap();
        assert_eq!(isolated.gain.into_base(), Complex64::default());
        relative(impedance(isolated).im, 4.);
    }
    let open = ideal(-2., SecondaryLoad::Open).unwrap().at(hz).unwrap();
    assert!(matches!(open.input_impedance, PortImpedance::Open));
    assert_eq!(open.gain.into_base(), Complex64::from(-2.));
    for k in [-1., 1.] {
        assert_eq!(
            pair(4., 1., k, load(0.)).unwrap_err(),
            MagneticError::ShortedIdeal
        );
    }
    assert_eq!(
        ideal(-2., load(0.)).unwrap_err(),
        MagneticError::ShortedIdeal
    );
}

#[test]
fn large_impedance_ratios_and_small_leakage_keep_finite_results() {
    let hz = Frequency::from_base(1. / TAU);
    // R/Xs underflows before it is multiplied by Xp in an unscaled evaluation.
    let p = pair(1e200, 1e200, 1., load(1e-200))
        .unwrap()
        .at(hz)
        .unwrap();
    relative(impedance(p).re, 1e-200);
    assert_eq!(p.gain.into_base(), Complex64::from(1.));
    for (n, r, expected) in [(1e200, 1e250, 1e-150), (1e-200, 1e-250, 1e150)] {
        relative(
            impedance(ideal(n, load(r)).unwrap().at(hz).unwrap()).re,
            expected,
        );
    }
    let k = f64::from_bits(1_f64.to_bits() - 1);
    let p = pair(2., 3., k, load(0.)).unwrap().at(hz).unwrap();
    relative(impedance(p).im, 2. * (1. - k) * (1. + k));
    // The short's leakage impedance fits even though the full reactance overflows.
    let p = pair(1e308, 1., k, load(0.))
        .unwrap()
        .at(Frequency::from_base(1.))
        .unwrap();
    relative(impedance(p).im, (1e308 * (1. - k) * (1. + k)) * TAU);
    // A short does not require the unrepresentable open-circuit turns ratio.
    let p = pair(1e-310, 1e308, 0.5, load(0.))
        .unwrap()
        .at(Frequency::from_base(1e100))
        .unwrap();
    relative(impedance(p).im, 0.75 * TAU * 1e-210);
    // Multiplying the turns ratio by k restores a finite open-circuit gain.
    let p = pair(1e-310, 1e308, 1e-20, SecondaryLoad::Open)
        .unwrap()
        .at(hz)
        .unwrap();
    relative(p.gain.into_base().re, 1e289);
}

#[test]
fn invalid_parameters_range_and_dc_are_reported() {
    for bad in [0., -1., f64::NAN, f64::INFINITY] {
        assert_eq!(
            pair(bad, 1., 0.5, load(1.)).unwrap_err(),
            MagneticError::Inductance
        );
        assert_eq!(
            pair(1., bad, 0.5, load(1.)).unwrap_err(),
            MagneticError::Inductance
        );
        let c = pair(1., 1., 1., load(1.)).unwrap();
        let i = ideal(1., load(1.)).unwrap();
        assert_eq!(
            c.at(Frequency::from_base(bad)).unwrap_err(),
            MagneticError::Frequency
        );
        assert_eq!(
            i.at(Frequency::from_base(bad)).unwrap_err(),
            MagneticError::Frequency
        );
    }
    for bad in [1.01, -1.01, f64::NAN, f64::INFINITY] {
        assert_eq!(
            pair(1., 2., bad, load(1.)).unwrap_err(),
            MagneticError::Coupling
        );
    }
    for bad in [0., f64::NAN, f64::INFINITY] {
        assert_eq!(ideal(bad, load(1.)).unwrap_err(), MagneticError::TurnsRatio);
    }
    for bad in [-1., f64::NAN, f64::INFINITY] {
        assert_eq!(
            pair(1., 1., 0., load(bad)).unwrap_err(),
            MagneticError::Load
        );
        assert_eq!(ideal(1., load(bad)).unwrap_err(), MagneticError::Load);
    }
    assert_eq!(
        ideal(1e300, load(1.)).unwrap_err(),
        MagneticError::DynamicRange
    );
    let c = pair(1e308, 1., 0., SecondaryLoad::Open).unwrap();
    assert_eq!(
        c.at(Frequency::from_base(1.)).unwrap_err(),
        MagneticError::DynamicRange
    );
}

#[test]
fn cli_errors_before_writing_a_sweep_for_invalid_models() {
    use std::process::Command;
    for args in [
        vec!["coupled", "10", "100", "3", "1", "4", "1", "0"],
        vec!["ideal", "10", "100", "3", "0", "open"],
        vec!["ideal", "10", "100", "3", "1", "40", "oops"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_electra-transformer"))
            .args(args)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}
