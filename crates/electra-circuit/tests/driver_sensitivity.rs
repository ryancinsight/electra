//! Digital-to-analog timing and independent divider sensitivity references.
use electra_circuit::{
    ElectricPotential, ElectricalImpedance,
    tolerance::{DividerSensitivity, DividerTolerance, ResistorRange},
    transient::{Capacitance, RcDriver, SeriesRc, Time},
};

#[test]
fn simultaneous_driver_edges_and_first_threshold_crossing() {
    let volts = ElectricPotential::from_base;
    let seconds = Time::from_base;
    let run = || {
        let load = SeriesRc::new(
            ElectricalImpedance::from_base(1000.),
            Capacitance::from_base(1e-6),
        )
        .unwrap();
        let mut driver = RcDriver::new(load, volts(0.), volts(0.)).unwrap();
        let start = 0.002;
        for rail in [5., 0., 3.] {
            assert_eq!(
                driver.update(seconds(start), volts(rail)).unwrap(),
                volts(0.)
            );
        }
        assert!(driver.update(seconds(start - 0.001), volts(0.)).is_err());
        let dt = 1e-6;
        for k in 1..2000 {
            let t = start + f64::from(k) * dt;
            if driver.update(seconds(t), volts(3.)).unwrap() >= volts(1.5) {
                return t;
            }
        }
        panic!("missing 1.5 V crossing")
    };
    let time = run();
    assert_eq!(time, run());
    let exact = 0.002 + 0.001 * std::f64::consts::LN_2;
    assert!((time - exact).abs() < 1e-6);
}

#[test]
fn adjoint_matches_closed_form_and_refined_central_difference() {
    let ohms = ElectricalImpedance::from_base;
    let volts = ElectricPotential::from_base;
    let response = |a, b| {
        let fixed = |r| ResistorRange::new(ohms(r), ohms(r)).unwrap();
        DividerTolerance::new(volts(9.), fixed(a), fixed(b))
            .unwrap()
            .corners()[0]
            .into_base()
    };
    let result = DividerSensitivity::at(volts(9.), ohms(1000.), ohms(2000.)).unwrap();
    for (upper, adjoint, exact) in [
        (true, result.upper.into_base(), -0.002),
        (false, result.lower.into_base(), 0.001),
    ] {
        assert!((adjoint - exact).abs() < 1e-17);
        let mut previous = None;
        for h in [16., 8., 4., 2.] {
            let difference = if upper {
                response(1000. + h, 2000.) - response(1000. - h, 2000.)
            } else {
                response(1000., 2000. + h) - response(1000., 2000. - h)
            };
            let error = (difference / (2. * h) - adjoint).abs();
            if let Some(old) = previous {
                assert!((3.99..4.01).contains(&(old / error)));
            }
            previous = Some(error);
        }
        assert!(previous.unwrap() / exact.abs() < 5e-7);
    }
}
