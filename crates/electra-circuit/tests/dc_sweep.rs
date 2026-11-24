//! Transfer-analysis contracts at the public deck, typed API and CLI boundaries.
use electra_circuit::{
    ElectricCurrent, ElectricPotential, LinearSweep, SourceValue, SweepError, SweepRange, parse,
};
use std::process::Command;

fn current(start: f64, stop: f64, step: f64) -> SweepRange {
    SweepRange::Current(LinearSweep {
        start: ElectricCurrent::from_base(start),
        stop: ElectricCurrent::from_base(stop),
        step: ElectricCurrent::from_base(step),
    })
}

#[test]
fn divider_transfer_reuses_probe_buffers_and_leaves_nominal_bias_available() {
    let c = parse(include_str!("../../../examples/transfer.cir")).unwrap();
    let mut run = c.dc_sweep(Default::default()).unwrap();
    assert_eq!(run.source_name(), "Vin");
    let mut buffers = None;
    for index in 0..=6 {
        let s = run.next_point().unwrap().unwrap();
        let vin = f64::from(index) / 2.;
        assert_eq!(
            s.excitation,
            SourceValue::Voltage(ElectricPotential::from_base(vin))
        );
        assert!((s.result.volts[2].1.into_base() - vin * 2.0 / 3.0).abs() < 1e-12);
        assert!((s.result.amperes[0].1.into_base() + vin / 3000.0).abs() < 1e-15);
        let addresses = (s.result.volts.as_ptr(), s.result.amperes.as_ptr());
        assert_eq!(*buffers.get_or_insert(addresses), addresses);
    }
    assert!(run.next_point().unwrap().is_none());
    assert!(run.next_point().unwrap().is_none());
    let nominal = c.operating_point(Default::default()).unwrap();
    assert!((nominal.volts[2].1.into_base() - 6.0).abs() < 1e-12);
}

#[test]
fn descending_current_transfer_preserves_small_bias_and_both_terminal_signs() {
    let c = parse("Ibig a b 1e25\nIbias 0 a 0.25\nR1 a 0 2\nR2 b 0 4").unwrap();
    let mut run = c
        .sweep_source("ibIG", current(1., -1., -0.5), Default::default())
        .unwrap();
    for i in [1., 0.5, 0., -0.5, -1.] {
        let s = run.next_point().unwrap().unwrap();
        assert_eq!(
            s.excitation,
            SourceValue::Current(ElectricCurrent::from_base(i))
        );
        assert_eq!(s.result.amperes[0].1.into_base(), i);
        assert_eq!(s.result.amperes[1].1.into_base(), 0.25);
        assert_eq!(s.result.volts[1].1.into_base(), 2. * (0.25 - i));
        assert_eq!(s.result.volts[2].1.into_base(), 4. * i);
    }
    assert!(run.next_point().unwrap().is_none());
}

#[test]
fn reversed_source_inclusive_decimal_endpoint_and_nondivisible_stop() {
    for (card, expected) in [
        ("0 0.3 0.1", vec![0., 0.1, 0.2, 0.3]),
        ("0.3 0 -0.1", vec![0.3, 0.2, 0.1, 0.]),
        ("0 0.31 0.1", vec![0., 0.1, 0.2, 0.3]),
        ("0.3 0 -0.2", vec![0.3, 0.1]),
        ("1 1 -5", vec![1.]),
        ("0 1 1e250", vec![0.]),
    ] {
        let c = parse(&format!(".DC vA {card}\nVa 0 a 0\nR1 a 0 1")).unwrap();
        let mut run = c.dc_sweep(Default::default()).unwrap();
        for value in expected {
            let s = run.next_point().unwrap().unwrap();
            assert!((s.result.volts[1].1.into_base() + value).abs() < 1e-15);
            assert!((s.result.amperes[0].1.into_base() + value).abs() < 1e-15);
        }
        assert!(run.next_point().unwrap().is_none());
    }
}

#[test]
fn invalid_cards_report_source_line_and_typed_api_rejects_wrong_source_kind() {
    for card in [
        ".dc",
        ".dc missing 0 1 1",
        ".dc R1 0 1 1",
        ".dc V1 0 1 -1",
        ".dc V1 0 1 0",
        ".dc V1 inf 1 1",
        ".dc V1 0 1 1e-25",
        ".dc V1 1h 1 1",
        ".dc V1 0 1 1 garbage",
    ] {
        let err = parse(&format!("V1 n 0 0\n\n{card}\nR1 n 0 1")).unwrap_err();
        assert_eq!(err.line, 3, "{card}");
    }
    assert_eq!(
        parse("V1 n 0 0\n.dc V1 0 0 1\n.dc V1 1 2 1")
            .unwrap_err()
            .line,
        3
    );
    let c = parse("V1 n 0 0").unwrap();
    assert!(matches!(
        c.dc_sweep(Default::default()),
        Err(SweepError::MissingDirective)
    ));
    assert!(matches!(
        c.sweep_source("V1", current(0., 1., 1.), Default::default()),
        Err(SweepError::SourceType(_))
    ));
}

#[test]
fn failed_point_reports_excitation_and_terminates_without_mutating_circuit() {
    let c = parse("V1 a 0 0\nR1 a 0 0.1\n.dc V1 0 1e308 1e308").unwrap();
    let mut run = c.dc_sweep(Default::default()).unwrap();
    assert!(run.next_point().unwrap().is_some());
    assert!(
        matches!(run.next_point(),Err(SweepError::Point(SourceValue::Voltage(v),_)) if v.into_base()==1e308)
    );
    assert!(run.next_point().unwrap().is_none());
    assert!(c.operating_point(Default::default()).is_ok());
}

#[test]
fn extreme_finite_span_and_unrepresentable_adjacent_values() {
    // The physical span exceeds f64::MAX, but its two unitless intervals are exact.
    let c = parse("V1 a 0 0\n.dc V1 -1e308 1e308 1e308").unwrap();
    let mut run = c.dc_sweep(Default::default()).unwrap();
    for expected in [-1e308, 0., 1e308] {
        assert_eq!(
            run.next_point().unwrap().unwrap().result.volts[1]
                .1
                .into_base(),
            expected
        );
    }
    assert!(run.next_point().unwrap().is_none());
    let c = parse("V1 a 0 0\n.dc V1 10000000000000000 10000000000000004 1").unwrap();
    let mut run = c.dc_sweep(Default::default()).unwrap();
    assert!(run.next_point().unwrap().is_some());
    assert!(matches!(run.next_point(), Err(SweepError::InvalidRange(_))));
    assert!(run.next_point().unwrap().is_none());
}

#[test]
fn cli_reports_all_seven_transfer_points_and_retains_op_semantics() {
    let deck = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/transfer.cir");
    let invoke = |mode| {
        let output = Command::new(env!("CARGO_BIN_EXE_electra"))
            .args([mode, deck])
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    };
    let transfer = invoke("dc");
    assert_eq!(
        transfer.lines().filter(|l| l.starts_with("Vin = ")).count(),
        7
    );
    assert!(transfer.contains("V(out) = 2.000000000000e0 V"));
    assert!(invoke("op").contains("V(out) = 6.000000000000e0 V"));
}

#[test]
fn spice_scale_factors_are_shared_by_cards_and_sweep_controls() {
    for (suffix, expected) in [
        ("T", 1e12),
        ("G", 1e9),
        ("mEg", 1e6),
        ("K", 1e3),
        ("M", 1e-3),
        ("u", 1e-6),
        ("N", 1e-9),
        ("p", 1e-12),
        ("F", 1e-15),
    ] {
        let c = parse(&format!(
            "I1 n 0 1e0{suffix}\nR1 n 0 1\n.dc I1 1{suffix} 1{suffix} 1"
        ))
        .unwrap();
        assert_eq!(
            c.operating_point(Default::default()).unwrap().volts[1]
                .1
                .into_base(),
            -expected
        );
        let mut s = c.dc_sweep(Default::default()).unwrap();
        assert_eq!(
            s.next_point().unwrap().unwrap().result.volts[1]
                .1
                .into_base(),
            -expected
        );
    }
    for invalid in ["2megohm", "1kk", "1e", "inf", "1e308T", "1e-310f", "1e-900"] {
        let bad = parse(&format!("I1 a 0 {invalid}")).unwrap_err();
        assert_eq!(bad.line, 1);
    }
}

#[test]
fn swept_bridge_with_a_second_supply_matches_the_two_by_two_solution() {
    // In node order (x, y) the matrix is [4,-2; -2,5]. The second supply is at -2 V.
    let c=parse("Vs plus 0 1\nVb 0 minus 2\nR1 plus x 1\nR2 x 0 1\nR3 x y 1\nR4 y x 1\nR5 minus y 0.5\nR6 y 0 1\nI1 0 x 0.125\n.dc Vs -3 3 0.25").unwrap();
    let mut run = c.dc_sweep(Default::default()).unwrap();
    for i in 0..=24 {
        let supply = -3.0 + f64::from(i) * 0.25;
        let rhs = supply + 0.125;
        let x = (5.0 * rhs - 8.0) / 16.0;
        let y = (2.0 * rhs - 16.0) / 16.0;
        let p = run.next_point().unwrap().unwrap();
        for ((_, actual), expected) in p.result.volts.iter().zip([0., supply, -2., x, y]) {
            assert!((actual.into_base() - expected).abs() < 2e-9);
        }
        let expected = [
            x - supply,
            2.0 * (-2.0 - y),
            supply - x,
            x,
            x - y,
            y - x,
            2.0 * (-2.0 - y),
            y,
            0.125,
        ];
        for ((_, actual), expected) in p.result.amperes.iter().zip(expected) {
            assert!((actual.into_base() - expected).abs() < 2e-9);
        }
    }
    assert!(run.next_point().unwrap().is_none());
}
