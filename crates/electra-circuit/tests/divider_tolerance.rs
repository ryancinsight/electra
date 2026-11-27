//! Exact corner and integer-reference histogram qualification.
use electra_circuit::{
    ElectricPotential, ElectricalImpedance,
    tolerance::{DividerTolerance, ResistorRange},
};
fn range(low: f64, high: f64) -> ResistorRange {
    ResistorRange::new(
        ElectricalImpedance::from_base(low),
        ElectricalImpedance::from_base(high),
    )
    .unwrap()
}
#[test]
fn corners_and_replayed_histogram_match_independent_reference() {
    let divider = DividerTolerance::new(
        ElectricPotential::from_base(9.),
        range(950., 1050.),
        range(1900., 2100.),
    )
    .unwrap();
    let expected = [6., 378. / 61., 342. / 59., 6.];
    for (v, reference) in divider.corners().into_iter().zip(expected) {
        assert!((v.into_base() - reference).abs() < 1e-14);
    }
    for _ in 0..2 {
        let mut bins = [0; 6];
        for v in divider.samples(2026, 16384) {
            let v = v.into_base();
            assert!((expected[2]..=expected[1]).contains(&v));
            let slot = [5.8, 5.9, 6., 6.1, 6.2].partition_point(|&edge| edge <= v);
            bins[slot] += 1;
        }
        assert_eq!(bins, [5, 2023, 6166, 6162, 2028, 0]);
    }
}
#[test]
fn invalid_ranges_fixed_values_and_extreme_ratio() {
    assert!(
        ResistorRange::new(
            ElectricalImpedance::default(),
            ElectricalImpedance::from_base(1.)
        )
        .is_none()
    );
    let divider = DividerTolerance::new(
        ElectricPotential::from_base(-f64::MAX),
        range(f64::MAX, f64::MAX),
        range(f64::MIN_POSITIVE, f64::MIN_POSITIVE),
    )
    .unwrap();
    assert_eq!(divider.samples(0, 0).len(), 0);
    assert_eq!(
        divider.samples(42, 1).next().unwrap().into_base(),
        -f64::MIN_POSITIVE
    );
}
