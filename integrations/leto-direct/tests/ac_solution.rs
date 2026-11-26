//! End-to-end comparisons with independent passive circuit solutions.
use electra_ac::{
    Capacitance, Complex64, ElectricConductance, ElectricalImpedance, Frequency, RcLadder, Stage,
};
use electra_circuit::parse_ac;
use electra_leto_direct::ac::solve;
use std::{f64::consts::TAU, path::Path, process::Command};

fn near(actual: Complex64, expected: Complex64) {
    let d = actual - expected;
    assert!(
        d.re.hypot(d.im) < 2e-12 * expected.re.hypot(expected.im).max(1e-15),
        "{actual:?} vs {expected:?}"
    );
}

#[test]
fn cli_solves_bridge_with_analytical_voltages_and_currents() {
    let out = Command::new(env!("CARGO_BIN_EXE_electra-ac-solve"))
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/ac-bridge.cir"))
        .arg(TAU.recip().to_string())
        .output()
        .unwrap();
    assert!(out.status.success(), "{:?}", out.stderr);
    let csv = String::from_utf8(out.stdout).unwrap();
    let v = |node: &str| {
        let row = csv
            .lines()
            .find(|s| s.split(',').next() == Some(node))
            .unwrap();
        let mut fields = row.split(',').skip(1);
        Complex64::new(
            fields.next().unwrap().parse().unwrap(),
            fields.next().unwrap().parse().unwrap(),
        )
    };
    let a = v("\"a\"");
    let b = v("\"b\"");
    let den = Complex64::new(6., 5.);
    near(a, Complex64::new(3., 2.) / den);
    near(b, Complex64::new(2., 2.) / den);
    near(
        Complex64::new(0., 1.) * (a - b),
        Complex64::new(0., 1.) / den,
    );
}

#[test]
fn two_pole_ladder_agrees_at_dc_and_across_frequency() {
    let deck = parse_ac("V1 in 0 AC 1\nR1 in a 1k\nC1 a 0 1u\nR2 a b 1k\nC2 b 0 1u").unwrap();
    let cells = [Stage::new(
        ElectricalImpedance::from_base(1000.),
        Capacitance::from_base(1e-6),
        ElectricConductance::default(),
    )
    .unwrap(); 2];
    let ladder = RcLadder::new(&cells).unwrap();
    for hz in [0., 0.01, 1., 100., 1000., 10000.] {
        let f = Frequency::from_base(hz);
        let point = solve(&deck, f).unwrap();
        near(
            point.volts[3].into_base(),
            ladder.at(f).unwrap().gain.into_base(),
        );
        assert!(point.residual.into_base() < 1e-15);
    }
}

#[test]
fn inductive_divider_and_fully_prescribed_deck() {
    let f = Frequency::from_base(TAU.recip());
    let deck = parse_ac("V1 in 0 AC 1 90\nR1 in out 1\nL1 out 0 1").unwrap();
    near(
        solve(&deck, f).unwrap().volts[2].into_base(),
        Complex64::new(-0.5, 0.5),
    );
    let fixed = parse_ac("V1 in 0 AC 2\nR1 in 0 1").unwrap();
    near(
        solve(&fixed, f).unwrap().volts[1].into_base(),
        Complex64::from(2.),
    );
}

#[test]
fn rejects_unanchored_resonant_and_oversize_systems() {
    let f = Frequency::from_base(TAU.recip());
    for text in ["R1 a b 1\nR2 b c 3\nR3 c a 7", "C1 n 0 1\nL1 n 0 1"] {
        assert!(solve(&parse_ac(text).unwrap(), f).is_err());
    }
    let text = (0..129)
        .map(|i| {
            format!(
                "R{i} n{i} {} 1\n",
                if i == 0 {
                    "0".into()
                } else {
                    format!("n{}", i - 1)
                }
            )
        })
        .collect::<String>();
    assert!(
        solve(&parse_ac(&text).unwrap(), f)
            .err()
            .unwrap()
            .to_string()
            .contains("exceeds")
    );
}

#[test]
fn separate_rows_can_have_extremely_different_admittance_scales() {
    let deck = parse_ac("V1 in 0 AC 1\nR1 in a 1e150\nR2 a 0 1e150\nR3 in b 1e-150\nR4 b 0 1e-150")
        .unwrap();
    let point = solve(&deck, Frequency::from_base(1.)).unwrap();
    near(point.volts[2].into_base(), Complex64::from(0.5));
    near(point.volts[3].into_base(), Complex64::from(0.5));
}

#[test]
fn physical_check_detects_loss_of_a_voltage_drop_in_the_solution() {
    let deck = parse_ac("V1 in 0 AC 1\nR1 in out 1e-18\nR2 out 0 1").unwrap();
    let err = solve(&deck, Frequency::from_base(1.))
        .err()
        .unwrap()
        .to_string();
    assert!(err.contains("physical AC KCL failed at out"), "{err}");
}

#[test]
fn original_branch_current_readback_keeps_direction_and_source_line() {
    let deck = parse_ac("V1 in 0 AC 1 90\nR1 in out 1\nL1 out 0 1").unwrap();
    let f = Frequency::from_base(TAU.recip());
    let point = solve(&deck, f).unwrap();
    let mut currents = Vec::new();
    deck.branch_currents(f, &point.volts, |line, ends, amps| {
        currents.push((line, ends, amps.into_base()))
    })
    .unwrap();
    assert_eq!(currents.len(), 2);
    assert_eq!((currents[0].0, currents[0].1), (2, [1, 2]));
    assert_eq!((currents[1].0, currents[1].1), (3, [2, 0]));
    for (_, _, z) in currents {
        near(z, Complex64::new(0.5, 0.5));
    }
}

#[test]
fn solves_many_interleaved_rc_channels_with_shared_drive() {
    let mut text = String::from("V in 0 AC 1\n");
    for i in 0..256 {
        text.push_str(&format!("R{i} in a{i} 1\n"));
    }
    for i in (0..256).rev() {
        text.push_str(&format!("Rs{i} a{i} b{i} 1\nC{i} b{i} 0 1\n"));
    }
    let deck = parse_ac(&text).unwrap();
    let point = solve(&deck, Frequency::from_base(TAU.recip())).unwrap();
    for (label, v) in deck.nodes().iter().zip(point.volts) {
        if label.starts_with('a') {
            near(v.into_base(), Complex64::new(0.6, -0.2));
        }
        if label.starts_with('b') {
            near(v.into_base(), Complex64::new(0.2, -0.4));
        }
    }
}

#[test]
fn a_dc_open_splits_a_chain_that_exceeds_the_ac_limit() {
    let mut text = String::from("V in 0 AC 1\n");
    for i in 0..129 {
        text.push_str(&format!("R{i} in n{i} 1\n"));
        if i > 0 {
            text.push_str(&format!("C{i} n{i} n{} 1\n", i - 1));
        }
    }
    let deck = parse_ac(&text).unwrap();
    assert!(
        solve(&deck, Frequency::from_base(0.)).unwrap().volts[2..]
            .iter()
            .all(|v| v.into_base() == Complex64::from(1.))
    );
    assert!(
        solve(&deck, Frequency::from_base(1.))
            .err()
            .unwrap()
            .to_string()
            .contains("exceeds")
    );
}
