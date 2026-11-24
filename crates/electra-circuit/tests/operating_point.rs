//! Analytical electrical and command-line operating points.
use electra_circuit::{SolveError, parse};
use std::{collections::HashMap, process::Command};

fn near<D>(actual: aequitas::Quantity<f64, D>, expected: f64, tolerance: f64) {
    let actual = actual.into_base();
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} vs {expected}"
    );
}

#[test]
fn divider_matches_voltages_signed_currents_and_power_balance() {
    let c = parse(include_str!("../../../examples/divider.cir")).unwrap();
    let result = c.operating_point(Default::default()).unwrap();
    let v: HashMap<_, _> = result.volts.into_iter().collect();
    let i: HashMap<_, _> = result.amperes.into_iter().collect();
    near(v["out"], 6., 1e-12);
    near(i["V1"], -0.003, 1e-15);
    near(i["R1"], 0.003, 1e-15);
    near(i["R2"], 0.003, 1e-15);
    let balance: aequitas::systems::si::quantities::Power =
        v["in"] * i["V1"] + (v["in"] - v["out"]) * i["R1"] + v["out"] * i["R2"];
    near(balance, 0., 1e-15);
}

#[test]
fn rail_drop_with_distributed_loads_matches_cumulative_ir() {
    let c = parse(include_str!("../../../examples/rail.cir")).unwrap();
    let result = c.operating_point(Default::default()).unwrap();
    let v: HashMap<_, _> = result.volts.into_iter().collect();
    let i: HashMap<_, _> = result.amperes.into_iter().collect();
    near(v["tap"], 4.97, 1e-10);
    near(v["far"], 4.95, 1e-10);
    near(i["Vrail"], -0.3, 1e-10);
}

#[test]
fn parallel_resistors_and_reversed_supply_preserve_signs() {
    let c = parse("Vsrc 0 a -1\nR1 a b 2\nR2 B A 2\nI1 b 0 1e-1\nI2 a 0 0.25").unwrap();
    let r = c.operating_point(Default::default()).unwrap();
    let i: HashMap<_, _> = r.amperes.into_iter().collect();
    near(i["Vsrc"], 0.35, 1e-14);
    near(i["R1"], 0.05, 1e-14);
    near(i["R2"], -0.05, 1e-14);
}

#[test]
fn unbalanced_bridge_matches_two_independent_nodal_equations() {
    let c = parse("V1 in 0 1\nR1 in a 1\nR2 a 0 1\nR3 in b 2\nR4 b 0 1\nR5 a b 1").unwrap();
    let r = c.operating_point(Default::default()).unwrap();
    let v: HashMap<_, _> = r.volts.into_iter().collect();
    // 3a-b=1, -a+2.5b=0.5.
    near(v["a"], 6. / 13., 1e-10);
    near(v["b"], 5. / 13., 1e-10);
}
#[test]
fn invalid_or_unsupported_inputs_fail_explicitly() {
    for text in [
        "",
        "V1 a b 3",
        "R1 a b 1k",
        "C1 a 0 1e-9",
        "R1 a a 10",
        "V1 a 0 1\nV2 a 0 1",
        ".end\nR1 a 0 1",
        "R1 a b 1\nr1 x y 2",
        "R1 a 0 0",
        "I1 a 0 inf",
    ] {
        assert!(parse(text).is_err(), "{text}");
    }
    let c = parse("V1 a 0 1\nR1 b c 100\nI1 a b 1").unwrap();
    assert!(matches!(
        c.operating_point(Default::default()),
        Err(SolveError::Floating(_))
    ));
}

#[test]
fn cli_runs_the_checked_in_deck() {
    let output = Command::new(env!("CARGO_BIN_EXE_electra"))
        .args([
            "op",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/divider.cir"),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("V(out) = 6.000000000000e0 V")
    );
}
