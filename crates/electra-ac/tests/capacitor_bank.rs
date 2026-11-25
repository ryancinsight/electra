//! Driving-point circuit laws for series RLC branches connected in parallel.
use electra_ac::pdn::{BankError, CapacitorBank, PortImpedance, SeriesCapacitor};
use electra_ac::{Capacitance, Complex64, ElectricalImpedance, Frequency, Inductance};
use std::f64::consts::TAU;

fn branch(c: f64, r: f64, l: f64) -> Result<SeriesCapacitor, BankError> {
    SeriesCapacitor::new(
        Capacitance::from_base(c),
        ElectricalImpedance::from_base(r),
        Inductance::from_base(l),
    )
}
fn finite(bank: CapacitorBank<'_>, hz: f64) -> Complex64 {
    match bank.at(Frequency::from_base(hz)).unwrap() {
        PortImpedance::Finite(z) => z.into_base(),
        PortImpedance::Open => panic!("expected finite impedance at {hz} Hz"),
    }
}

#[test]
fn open_short_and_parallel_antiresonance_are_distinct() {
    let f = TAU.recip(); // ω = 1 rad/s exactly for these fixtures
    let hz = Frequency::from_base(f);
    let capacitive = branch(1., 0., 0.).unwrap(); // −j Ω
    let inductive = branch(1., 0., 2.).unwrap(); // +j Ω
    let short = branch(1., 0., 1.).unwrap();
    assert!(matches!(
        CapacitorBank::new(&[]).at(hz).unwrap(),
        PortImpedance::Open
    ));
    assert!(matches!(
        CapacitorBank::new(&[short])
            .at(Frequency::default())
            .unwrap(),
        PortImpedance::Open
    ));
    assert!(matches!(
        CapacitorBank::new(&[capacitive, inductive]).at(hz).unwrap(),
        PortImpedance::Open
    ));
    for parts in [[capacitive, short], [short, capacitive]] {
        assert_eq!(finite(CapacitorBank::new(&parts), f), Complex64::default());
    }
    let weak_loss = branch(1., 1e-18, 0.).unwrap();
    let z = finite(CapacitorBank::new(&[inductive, weak_loss]), f);
    assert!((z.re / 1e18 - 1.).abs() < 1e-14);
}

#[test]
fn series_resonance_and_quality_factor_from_current_bandwidth() {
    let (c, r, l): (f64, f64, f64) = (1e-6, 2., 100e-6);
    let parts = [branch(c, r, l).unwrap()];
    let bank = CapacitorBank::new(&parts);
    let f0 = (TAU * (l * c).sqrt()).recip();
    let q = (l / c).sqrt() / r;
    // Locate the reactance zero and both half-power points from the response.
    let crossing = |mut lo: f64, mut hi: f64, threshold: f64| {
        for _ in 0..64 {
            let mid = (lo + hi) / 2.;
            if finite(bank, mid).im < threshold {
                lo = mid
            } else {
                hi = mid
            }
        }
        (lo + hi) / 2.
    };
    let center = crossing(f0 / 2., 2. * f0, 0.);
    let lower = crossing(f0 / 2., center, -r);
    let upper = crossing(center, 2. * f0, r);
    assert!((center / f0 - 1.).abs() < 1e-12);
    assert!((center / (upper - lower) / q - 1.).abs() < 1e-12);
    for f in [lower, upper] {
        let z = finite(bank, f);
        assert!((z.re.hypot(z.im) / (r * 2_f64.sqrt()) - 1.).abs() < 1e-12);
    }
}

#[test]
fn scaled_reciprocals_and_cancellation_preserve_finite_ports() {
    let f = TAU.recip();
    for r in [1e-250, 1e250] {
        let cap = branch(1., r, 1.).unwrap();
        let z = finite(CapacitorBank::new(&[cap, cap]), f);
        assert!((z.re / (r / 2.) - 1.).abs() < 1e-14);
        assert_eq!(z.im, 0.);
    }
    // A small susceptance between +j and −j must survive cancellation.
    let plus = branch(1., 0., 0.).unwrap();
    let minus = branch(1., 0., 2.).unwrap();
    let small = branch(1e-24, 0., 0.).unwrap();
    for parts in [
        [plus, small, minus],
        [minus, plus, small],
        [small, minus, plus],
    ] {
        let z = finite(CapacitorBank::new(&parts), f);
        assert_eq!(z.re, 0.);
        assert!((z.im / -1e24 - 1.).abs() < 1e-14);
    }
}

#[test]
fn validation_and_dynamic_range_preserve_ideal_short_precedence() {
    for value in [0., -1., f64::INFINITY, f64::NAN] {
        assert_eq!(branch(value, 1., 1.).unwrap_err(), BankError::Capacitance);
    }
    for value in [-1., f64::INFINITY, f64::NAN] {
        assert_eq!(branch(1., value, 1.).unwrap_err(), BankError::Resistance);
        assert_eq!(branch(1., 1., value).unwrap_err(), BankError::Inductance);
        assert_eq!(
            CapacitorBank::new(&[])
                .at(Frequency::from_base(value))
                .unwrap_err(),
            BankError::Frequency
        );
    }
    let tiny = branch(f64::from_bits(1), 0., 0.).unwrap();
    let f = TAU.recip();
    assert_eq!(
        CapacitorBank::new(&[tiny])
            .at(Frequency::from_base(f))
            .unwrap_err(),
        BankError::DynamicRange
    );
    let short = branch(1., 0., 1.).unwrap();
    for parts in [[short, tiny], [tiny, short]] {
        assert_eq!(finite(CapacitorBank::new(&parts), f), Complex64::default());
    }
}
