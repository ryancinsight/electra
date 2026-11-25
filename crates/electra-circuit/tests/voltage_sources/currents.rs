use super::near;
use electra_circuit::parse;
use std::collections::HashMap;
use std::process::Command;

#[test]
fn voltage_and_zero_volt_probe_polarity_survive_reversals() {
    for (source, probe, voltage, probe_sign) in [
        ("a b", "b tap", 3., 1.),
        ("b a", "b tap", 3., 1.),
        ("a b", "tap b", -3., -1.),
    ] {
        let c = parse(&format!(
            "Vcell {source} {voltage}\nVprobe {probe} 0\nRa a 0 1k\nRb tap 0 2k"
        ))
        .unwrap();
        let p = c.operating_point(Default::default()).unwrap();
        let nodes: HashMap<_, _> = p.volts.into_iter().collect();
        let orientation = if source == "a b" { 1. } else { -1. };
        let current = orientation * voltage / 3000.;
        near(nodes["a"], 1000. * current);
        near(nodes["b"], -2000. * current);
        near(nodes["tap"], -2000. * current);
        near(p.amperes[0].1, -voltage / 3000.);
        near(p.amperes[1].1, -current * probe_sign);
    }
}

#[test]
fn internal_and_external_current_sweeps_recover_each_branch_reaction() {
    for (terminals, internal) in [("a b", true), ("0 b", false)] {
        let c = parse(&format!(
            "V1 a b 3\nR1 a 0 1k\nR2 b 0 2k\nI1 {terminals} 1e25\n.dc I1 -1m 1m 1m"
        ))
        .unwrap();
        let mut sweep = c.dc_sweep(Default::default()).unwrap();
        for i in [-0.001, 0., 0.001] {
            let p = sweep.next_point().unwrap().unwrap().result;
            let va = if internal { 1. } else { (2000. * i + 3.) / 3. };
            near(p.volts[1].1, va);
            near(p.volts[2].1, va - 3.);
            near(p.amperes[0].1, -va / 1000. - if internal { i } else { 0. });
            near(p.amperes[3].1, i);
        }
        assert!(sweep.next_point().unwrap().is_none());
    }
}

#[test]
fn branching_tree_reactions_include_parallel_resistors_and_an_internal_current_source() {
    // Assign ground-referenced voltages and solve the edge currents by hand at the leaves.
    let c=parse("Vleft n root 2\nVright root m 1\nVrail root 0 5\nVleaf t n -4\nR1 n m 3\nR2 m n 6\nRt t 0 3\nRm m 0 2\nIx t m 0.25\nIr 0 root 0.125").unwrap();
    let r = c.operating_point(Default::default()).unwrap();
    let currents: HashMap<_, _> = r.amperes.into_iter().collect();
    for (name, expected) in [
        ("Vleaf", -1.25),
        ("Vleft", -2.75),
        ("Vright", 0.25),
        ("Vrail", -2.875),
        ("R1", 1.),
        ("R2", -0.5),
    ] {
        near(currents[name], expected);
    }
}

#[test]
fn cli_sweeps_the_floating_cell_and_signed_probe() {
    let result = Command::new(env!("CARGO_BIN_EXE_electra"))
        .args([
            "dc",
            concat!(env!("CARGO_MANIFEST_DIR"), "/examples/floating-source.cir"),
        ])
        .output()
        .unwrap();
    assert!(result.status.success());
    let text = String::from_utf8(result.stdout).unwrap();
    assert_eq!(text.lines().filter(|l| l.starts_with("Vcell =")).count(), 7);
    assert!(text.contains("I(Vprobe) = -1.000000000000e-3 A"));
    assert!(text.contains("I(Vprobe) = 1.000000000000e-3 A"));
}

#[test]
fn cli_accepts_typed_limits_and_rejects_ambiguous_or_invalid_options() {
    let deck = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/floating-source.cir");
    let run = |flags: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_electra"))
            .args(["dc", deck])
            .args(flags)
            .output()
            .unwrap()
    };
    assert!(
        run(&[
            "--atol-amps",
            "1e-14",
            "--rtol",
            "1e-13",
            "--max-sweeps",
            "4"
        ])
        .status
        .success()
    );
    for flags in [
        &["--rtol", "1"][..],
        &["--atol-amps", "-1"],
        &["--max-sweeps", "0"],
        &["--atol-amps", "1", "--atol-amps", "2"],
        &["--rtol"],
        &["--oops", "1"],
    ] {
        assert!(!run(flags).status.success());
    }
}

#[test]
fn floating_voltage_sweep_retains_independent_current_bias_at_each_point() {
    let c = parse("Vcell a b 8\nR1 a 0 1k\nR2 b 0 2k\nIbias 0 b 1m\n.dc Vcell 0 3 1").unwrap();
    let mut sweep = c.dc_sweep(Default::default()).unwrap();
    for v in [0., 1., 2., 3.] {
        let p = sweep.next_point().unwrap().unwrap().result;
        let a = (2. + v) / 3.;
        near(p.volts[1].1, a);
        near(p.volts[2].1, a - v);
        near(p.amperes[0].1, -a / 1000.);
    }
    assert!(sweep.next_point().unwrap().is_none());
}
