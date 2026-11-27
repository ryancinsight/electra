//! Mixed-signal timing and sensitivity qualification.
use electra_circuit::{
    ElectricPotential, ElectricalImpedance,
    tolerance::{DividerSensitivity, DividerTolerance, ResistorRange},
    transient::{Capacitance, RcDriver, SchmittReceiver, SeriesRc, Time},
};

#[test]
fn rc_drive_hysteresis_and_delay() {
    let v = ElectricPotential::from_base;
    let t = Time::from_base;
    let rc = SeriesRc::new(
        ElectricalImpedance::from_base(1000.),
        Capacitance::from_base(1e-6),
    )
    .unwrap();
    let mut driver = RcDriver::new(rc, v(0.), v(0.)).unwrap();
    for rail in [3., 0., 3.] {
        driver.update(t(0.), v(rail)).unwrap();
    }
    let mut receiver = SchmittReceiver::new(v(1.), v(2.), t(0.0001), false).unwrap();
    let mut edges = Vec::new();
    for k in 1..6001 {
        let time = t(f64::from(k) * 1e-6);
        let voltage = driver
            .update(time, v(if k < 3000 { 3. } else { 0. }))
            .unwrap();
        if let Some(edge) = receiver.observe(time, voltage).unwrap() {
            edges.push(edge);
        }
    }
    let falling = 0.003 + 0.001 * (3. * (1. - (-3.0_f64).exp())).ln();
    assert_eq!(edges.len(), 2);
    for (edge, (level, crossing)) in edges
        .iter()
        .zip([(true, 0.001 * 3.0_f64.ln()), (false, falling)])
    {
        assert_eq!(edge.high, level);
        assert!((edge.time.into_base() - crossing - 0.0001).abs() < 2e-6);
    }
}

#[test]
fn pulse_cancellation_and_simultaneous_deadline_order() {
    let v = ElectricPotential::from_base;
    let t = Time::from_base;
    let mut r = SchmittReceiver::new(v(1.), v(2.), t(2.), false).unwrap();
    assert!(r.observe(t(0.), v(2.)).unwrap().is_none());
    assert_eq!(r.deadline(), Some(t(2.)));
    assert!(r.observe(t(1.), v(1.)).unwrap().is_none());
    assert_eq!(r.deadline(), None);
    r.observe(t(2.), v(2.)).unwrap();
    r.observe(t(3.), v(1.5)).unwrap();
    let rise = r.observe(t(4.), v(1.)).unwrap().unwrap();
    assert!(rise.high);
    assert_eq!(rise.time, t(4.));
    let fall = r.observe(t(8.), v(1.5)).unwrap().unwrap();
    assert!(!fall.high);
    assert_eq!(fall.time, t(6.));
}

#[test]
fn linear_resistor_sweep_and_seed_replay() {
    let ohm = ElectricalImpedance::from_base;
    let drive = ElectricPotential::from_base(9.);
    let range = |a, b| ResistorRange::new(ohm(a), ohm(b)).unwrap();
    let experiment = DividerTolerance::new(drive, range(999., 1001.), range(2000., 2000.)).unwrap();
    assert!(
        experiment
            .samples(2026, 4096)
            .eq(experiment.samples(2026, 4096))
    );
    let sensitivity = DividerSensitivity::at(drive, ohm(1000.), ohm(2000.))
        .unwrap()
        .upper
        .into_base();
    assert!((sensitivity + 0.002).abs() < 1e-16);
    for k in -10..=10 {
        let delta = f64::from(k) / 10.;
        let actual = DividerTolerance::new(
            drive,
            range(1000. + delta, 1000. + delta),
            range(2000., 2000.),
        )
        .unwrap()
        .corners()[0]
            .into_base();
        assert!((actual - (6. + delta * sensitivity)).abs() < 6.7e-7);
    }
}
