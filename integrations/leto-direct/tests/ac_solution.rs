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
    let mut text = String::new();
    // A 130-node circulant graph has degree four everywhere.
    for i in 0..130 {
        for step in [1, 2] {
            text.push_str(&format!("R{i}_{step} n{i} n{} 1\n", (i + step) % 130));
        }
    }
    text.push_str("Rground n0 0 1");
    assert!(
        solve(&parse_ac(&text).unwrap(), f)
            .err()
            .unwrap()
            .to_string()
            .contains("irreducible AC core")
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
fn a_dc_open_splits_a_chain_that_uses_the_tree_solver_at_ac() {
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
    let point = solve(&deck, Frequency::from_base(1.)).unwrap();
    for v in &point.volts[2..] {
        near(v.into_base(), Complex64::from(1.));
    }
}

#[test]
fn large_branched_star_matches_loaded_node_solution() {
    let mut text = String::from("Vdrive drive 0 AC 1\n");
    // Intern a leaf first so traversal starts away from the branching hub.
    for i in 0..200 {
        text.push_str(&format!("R{i} leaf{i} hub 1\nC{i} leaf{i} 0 1\n"));
    }
    text.push_str("Rdrive drive hub 1\n");
    let deck = parse_ac(&text).unwrap();
    let point = solve(&deck, Frequency::from_base(TAU.recip())).unwrap();
    let hub = Complex64::from(1.) / Complex64::new(101., 100.);
    let leaf = hub / Complex64::new(1., 1.);
    for (node, v) in deck.nodes().iter().zip(point.volts) {
        if node.starts_with("leaf") {
            near(v.into_base(), leaf);
        }
        if node == "hub" {
            near(v.into_base(), hub);
        }
    }
}

#[test]
fn resonant_leaf_constraint_is_solved() {
    let mut text = String::from("Vdrive drive 0 AC 1\nRdrive drive n0 1\n");
    for i in 1..129 {
        text.push_str(&format!("R{i} n{} n{i} 1\n", i - 1));
    }
    text.push_str("Cleaf n128 leaf 1\nLleaf leaf 0 1\n");
    let deck = parse_ac(&text).unwrap();
    let point = solve(&deck, Frequency::from_base(TAU.recip())).unwrap();
    for (label, v) in deck.nodes().iter().zip(&point.volts) {
        if label == "leaf" {
            near(v.into_base(), Complex64::new(0., 1. / 129.));
        }
        if label == "n128" {
            assert!(v.into_base().re.hypot(v.into_base().im) < 1e-13);
        }
    }
}

#[test]
fn driven_ring_matches_both_analytical_resistor_arms() {
    let mut text = String::from("Vdrive in 0 AC 1 90\nRsource in n0 1\nRload n128 0 1\n");
    for i in 0..256 {
        text.push_str(&format!("R{i} n{i} n{} 1\n", (i + 1) % 256));
    }
    let deck = parse_ac(&text).unwrap();
    let point = solve(&deck, Frequency::from_base(1.)).unwrap();
    for (name, v) in deck.nodes().iter().zip(point.volts) {
        if let Some(index) = name.strip_prefix('n') {
            let i: usize = index.parse().unwrap();
            let distance = i.min(256 - i) as f64;
            near(
                v.into_base(),
                Complex64::new(0., (65. - distance / 2.) / 66.),
            );
        }
    }
}

#[test]
fn series_reduction_preserves_a_multicycle_pivoted_core() {
    let core = "V1 drive 0 AC 1 90\nRin drive a 4\nRac a c 2\nRbc b c 3\nRad a d 5\nRbd b d 7\nRcd c d 9\nRload b 0 1\nCload d 0 0.1\n";
    let f = Frequency::from_base(0.75);
    let reference = parse_ac(&format!("{core}Rab a b 150")).unwrap();
    let exact = solve(&reference, f).unwrap();
    let voltage = |name: &str| {
        exact.volts[reference.nodes().iter().position(|n| n == name).unwrap()].into_base()
    };
    let mut text = String::from(core);
    let mut last = String::from("a");
    for i in 1..=150 {
        let next = if i == 150 {
            "b".to_owned()
        } else {
            format!("n{i}")
        };
        text.push_str(&format!("Rs{i} {last} {next} 1\n"));
        last = next;
    }
    let deck = parse_ac(&text).unwrap();
    let point = solve(&deck, f).unwrap();
    for (name, v) in deck.nodes().iter().zip(point.volts) {
        let expected = if let Some(n) = name.strip_prefix('n') {
            voltage("a")
                + (voltage("b") - voltage("a")) * Complex64::from(n.parse::<f64>().unwrap() / 150.)
        } else {
            voltage(name)
        };
        near(v.into_base(), expected);
    }
}
