use electra_circuit::{
    ElectricPotential, LinearSweep, MnaError, MnaOptions, SweepError, SweepRange, parse,
};

#[test]
fn singular_zero_rhs_is_rejected_before_krylov_and_budget_failure_terminates_sweep() {
    let floating = parse("R1 a b 2\nIzero a 0 0").unwrap();
    assert!(matches!(
        floating.operating_point_mna(Default::default()),
        Err(MnaError::Floating(1))
    ));
    let c = parse("V1 a 0 0\nR1 a b 1k\nR2 b 0 2k\n.dc V1 0 2 1").unwrap();
    let mut s = c
        .dc_sweep_mna(MnaOptions {
            max_iterations: 1,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(s.next_point().unwrap().unwrap().result.iterations, 0);
    let err = s.next_point().err().unwrap();
    assert!(
        matches!(err,MnaError::Point(_,inner) if matches!(*inner,MnaError::Krylov{iterations:1,..}))
    );
    assert!(s.next_point().unwrap().is_none());
}

#[test]
fn unrepresentable_constraint_and_erased_nonzero_stamps_are_diagnosed() {
    let c = parse("Vbase a 0 1e16\nVsmall b a 1\nR1 b 0 1e16").unwrap();
    assert!(c.operating_point_mna(Default::default()).is_err());
    let c = parse("Vbase a 0 1\nRweak a b 1e300\nRstrong b 0 1e-300").unwrap();
    assert!(matches!(
        c.operating_point_mna(Default::default()),
        Err(MnaError::Arithmetic)
    ));
}
#[test]
fn invalid_tolerances_and_wrong_typed_source_are_rejected() {
    let c = parse("I1 a 0 1\nR a 0 1").unwrap();
    for absolute in [f64::NAN, f64::INFINITY, -1.] {
        assert!(matches!(
            c.operating_point_mna(MnaOptions {
                voltage_absolute: ElectricPotential::from_base(absolute),
                ..Default::default()
            }),
            Err(MnaError::InvalidOptions)
        ));
    }
    let v = ElectricPotential::from_base(1.);
    assert!(matches!(
        c.sweep_source_mna(
            "I1",
            SweepRange::Voltage(LinearSweep {
                start: v,
                stop: v,
                step: v
            }),
            Default::default()
        ),
        Err(MnaError::Sweep(SweepError::SourceType(_)))
    ));
}
