//! Compare a nominal electrical load against Kwavers resonance landmarks and an independent branch-current sweep.
use electra_ac::{piezo::BvdLoad, *};
use kwavers_driver::{bvd_anti_resonance_hz, bvd_series_resonance_hz};
use std::f64::consts::TAU;
#[test]
fn nominal_one_mhz_electrical_port() {
    let (rm, cm, c0, vrms) = (8., 100e-12, 1e-9, 2.);
    let lm = 1. / ((TAU * 1e6).powi(2) * cm);
    let model = BvdLoad::new(
        ElectricalImpedance::from_base(rm),
        Inductance::from_base(lm),
        Capacitance::from_base(cm),
        Capacitance::from_base(c0),
    )
    .unwrap();
    assert!(matches!(
        model.at(Frequency::default()),
        Ok(PortImpedance::Open)
    ));
    let mut trough = (0., f64::INFINITY);
    let mut peak = (0., 0.);
    for k in 0..=20000 {
        let f = 950000. + f64::from(k) * 10.;
        let w = TAU * f;
        let x = w * lm - 1. / (w * cm);
        let motional_i2 = vrms * vrms / (rm * rm + x * x);
        let ir = motional_i2 * rm / vrms;
        let iq = w * c0 * vrms - motional_i2 * x / vrms;
        let expected = vrms / ir.hypot(iq);
        let PortImpedance::Finite(z) = model.at(Frequency::from_base(f)).unwrap() else {
            panic!("finite port")
        };
        let z = z.into_base();
        let mag = z.norm();
        assert!((mag / expected - 1.).abs() < 1e-9);
        let power = vrms * vrms * z.re / z.norm_sqr();
        assert!((power / (rm * motional_i2) - 1.).abs() < 1e-9);
        if mag < trough.1 {
            trough = (f, mag);
        }
        if mag > peak.1 {
            peak = (f, mag);
        }
    }
    assert!((trough.0 / bvd_series_resonance_hz(lm, cm) - 1.).abs() < 0.001);
    assert!((peak.0 / bvd_anti_resonance_hz(lm, cm, c0) - 1.).abs() < 0.001);
    println!(
        "minimum {trough:?}; maximum {peak:?}; series power {} W",
        vrms * vrms / rm
    );
}
