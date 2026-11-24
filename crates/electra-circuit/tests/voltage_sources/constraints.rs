use electra_circuit::{SolveError, SweepError, parse};

#[test]
fn voltage_loops_distinguish_incompatible_values_from_undetermined_source_currents() {
    for (deck, line, source, kind) in [
        (
            "V1 a 0 1\nV2 a 0 1.0000000000000002",
            2,
            "V2",
            "incompatible",
        ),
        ("V1 a 0 2\nV2 0 a -2", 2, "V2", "redundant"),
        ("V1 a 0 2\nV2 0 a 2", 2, "V2", "incompatible"),
        ("V1 a b 0\nVprobe b a 0", 2, "Vprobe", "redundant"),
        ("V1 a b 0.1\nV2 b c 0.2\nV3 a c 0.3", 3, "V3", "redundant"),
        (
            "V1 a b 0.1\nV2 b c 0.2\nV3 a c 0.301",
            3,
            "V3",
            "incompatible",
        ),
        (
            "V1 a 0 2\nV2 a 0 2\nV3 a b 1\nV4 b 0 9",
            4,
            "V4",
            "incompatible",
        ),
        (
            "V0 g 0 1e16\nV1 a g 1\nV2 b a 1\n* relative to g\nV3 b g 3",
            5,
            "V3",
            "incompatible",
        ),
    ] {
        let e = parse(deck).unwrap_err();
        assert_eq!(e.line, line);
        assert!(
            e.message.contains(kind) && e.message.contains(source),
            "{e}"
        );
    }
}

#[test]
fn a_floating_source_and_a_floating_island_are_distinct() {
    let c = parse("Vground rail 0 2\nV1 a b 3\nR1 b c 1\nI1 rail a 1m\n.dc V1 0 1 1").unwrap();
    assert!(matches!(
        c.operating_point(Default::default()),
        Err(SolveError::Floating(2))
    ));
    assert!(matches!(
        c.dc_sweep(Default::default()),
        Err(SweepError::Network(SolveError::Floating(2)))
    ));
    let c = parse("V1 a b 3\nRb b 0 4").unwrap();
    let point = c.operating_point(Default::default()).unwrap();
    super::near(point.volts[1].1, 3.);
    super::near(point.amperes[0].1, 0.);
}

#[test]
fn unrepresentable_voltage_differences_and_nonfinite_original_currents_fail() {
    // Two distinct nodes round to the same binary64 absolute voltage.
    let c = parse("Vbase bias 0 1e16\nV1 a bias 1\n.dc V1 0 1 1").unwrap();
    assert!(matches!(
        c.operating_point(Default::default()),
        Err(SolveError::Arithmetic)
    ));
    let mut sweep = c.dc_sweep(Default::default()).unwrap();
    assert!(sweep.next_point().unwrap().is_some());
    assert!(matches!(
        sweep.next_point(),
        Err(SweepError::Point(_, SolveError::Arithmetic))
    ));
    assert!(sweep.next_point().unwrap().is_none());
    // Both resistor ends are in one supernode, so only reconstruction sees overflow.
    let c = parse("V1 a 0 0\nV2 b a 1e308\nR1 a b 0.1").unwrap();
    assert!(matches!(
        c.operating_point(Default::default()),
        Err(SolveError::Arithmetic)
    ));
}

#[test]
fn deep_source_chains_use_an_iterative_traversal() {
    use std::fmt::Write;
    let mut deck = String::new();
    for i in 1..=20000 {
        writeln!(deck, "V{i} {i} {} 1", i - 1).unwrap();
    }
    writeln!(deck, "Rload 20000 0 20k").unwrap();
    let c = parse(&deck).unwrap();
    let point = c.operating_point(Default::default()).unwrap();
    super::near(point.volts.last().unwrap().1, 20000.);
    for &(_, i) in &point.amperes[..20000] {
        super::near(i, -1.);
    }
}
