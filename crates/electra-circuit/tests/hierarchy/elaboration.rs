use electra_circuit::{DcResult, parse, parse_file};
use std::fmt::Write;

fn volt(p: &DcResult<'_>, name: &str) -> f64 {
    p.volts
        .iter()
        .find(|(n, _)| *n == name)
        .unwrap()
        .1
        .into_base()
}

fn current(p: &DcResult<'_>, name: &str) -> f64 {
    p.amperes
        .iter()
        .find(|(n, _)| *n == name)
        .unwrap()
        .1
        .into_base()
}

fn close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} vs {expected}"
    );
}

fn compare_dividers(h: &DcResult<'_>, flat: &DcResult<'_>, rail: f64) {
    let nodes = [
        ("0", "0", 0.),
        ("rail", "rail", rail),
        ("left", "left", rail * 2. / 3.),
        ("Xleft/junction", "j1", rail * 2. / 3.),
        ("right", "right", rail / 3.),
        ("Xright/junction", "j2", rail / 3.),
    ];
    assert_eq!(h.volts.len(), nodes.len());
    for (name, other, expected) in nodes {
        let v = volt(h, name);
        close(v, volt(flat, other), 1e-12);
        close(v, expected, 1e-12);
    }
    let parts = [
        ("Vin", "Vin", -2.),
        ("Xleft/Rupper", "Ru1", 1.),
        ("Xleft/Vsense", "Vs1", 1.),
        ("Xleft/Rlower", "Rl1", 1.),
        ("Xright/Rupper", "Ru2", 1.),
        ("Xright/Vsense", "Vs2", 1.),
        ("Xright/Rlower", "Rl2", 1.),
    ];
    assert_eq!(h.amperes.len(), parts.len());
    for (name, other, sign) in parts {
        let i = current(h, name);
        close(i, current(flat, other), 1e-15);
        close(i, sign * rail / 3000., 1e-15);
    }
}

#[test]
fn public_example_matches_every_flat_probe_and_closed_form_at_bias_and_each_sweep_point() {
    let file = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/hierarchical-divider.cir"
    );
    let h = parse_file(file).unwrap();
    let f = parse("Vin rail 0 9\nRu1 rail j1 1k\nVs1 j1 left 0\nRl1 left 0 2k\nRu2 rail j2 2k\nVs2 j2 right 0\nRl2 right 0 1k\n.dc Vin 0 9 3").unwrap();
    compare_dividers(
        &h.operating_point(Default::default()).unwrap(),
        &f.operating_point(Default::default()).unwrap(),
        9.,
    );
    let mut hs = h.dc_sweep(Default::default()).unwrap();
    let mut fs = f.dc_sweep(Default::default()).unwrap();
    for rail in [0., 3., 6., 9.] {
        compare_dividers(
            hs.next_point().unwrap().unwrap().result,
            fs.next_point().unwrap().unwrap().result,
            rail,
        );
    }
    assert!(hs.next_point().unwrap().is_none() && fs.next_point().unwrap().is_none());
}

#[test]
fn nested_forward_references_local_shadowing_auto_units_and_qualified_sweep() {
    let h = parse("Xone a wrapper r={R}\nXtwo b WRAPPER\n.param r=2kOhm\n.subckt wrapper out params: r=4kOhm\nXinner out source drive={drive} load={r}\n.param drive=6V\n.ends\n.subckt source pin params: load=1Ohm drive=12V\nVsrc pin 0 {drive}\nRload pin 0 {load}\n.ends\n.dc xOne/Xinner/vSRC 0 4 2").unwrap();
    let nominal = h.operating_point(Default::default()).unwrap();
    close(current(&nominal, "Xone/Xinner/Rload"), 0.003, 1e-15);
    close(current(&nominal, "Xtwo/Xinner/Rload"), 0.0015, 1e-15);
    let mut sweep = h.dc_sweep(Default::default()).unwrap();
    for v in [0., 2., 4.] {
        let p = sweep.next_point().unwrap().unwrap().result;
        close(volt(p, "a"), v, 1e-12);
        close(volt(p, "b"), 6., 1e-12);
        close(current(p, "Xone/Xinner/Vsrc"), -v / 2000., 1e-15);
        close(current(p, "Xtwo/Xinner/Vsrc"), -0.0015, 1e-15);
    }
}

#[test]
fn local_current_parameter_and_unused_forwarded_pin() {
    let h = parse(".param bias=1A\nXload rail unused top\nV1 rail 0 5\n.subckt top p spare\nX1 p spare sink\n.ends\n.subckt sink p spare\nIload p 0 {bias}\n.param bias=2mA\n.ends").unwrap();
    let p = h.operating_point(Default::default()).unwrap();
    assert_eq!(p.volts.len(), 2);
    close(current(&p, "V1"), -0.002, 1e-15);
}

#[test]
fn deeply_nested_models_elaborate_and_solve_without_call_stack_recursion() {
    let mut s = String::from("V1 a 0 5\nXtop a m0\n");
    for index in 0..4096 {
        writeln!(s, ".subckt m{index} p\nXnext p m{}\n.ends", index + 1).unwrap();
    }
    s.push_str(".subckt m4096 p\nRload p 0 2k\n.ends");
    let h = parse(&s).unwrap();
    let p = h.operating_point(Default::default()).unwrap();
    close(current(&p, "V1"), -0.0025, 1e-15);
    assert!(p.amperes[1].0.ends_with("/Rload"));
}

#[test]
fn parameterized_resistance_accepts_inherited_scale_and_sibling_instance_names() {
    let h = parse("V1 a 0 8\nXone a cell r=2k\nXtwo a cell r=4kOhm\n.subckt CELL p params: r=1Ohm\nRload p 0 {R}\n.ends cell").unwrap();
    let p = h.operating_point(Default::default()).unwrap();
    close(current(&p, "Xone/Rload"), 0.004, 1e-15);
    close(current(&p, "Xtwo/Rload"), 0.002, 1e-15);
}

#[test]
fn exponential_empty_subtrees_are_skipped_and_unrepresentable_physical_counts_are_diagnosed() {
    let mut definitions = String::new();
    for i in 0..usize::BITS {
        writeln!(
            definitions,
            ".subckt t{i} n\nXa n t{}\nXb n t{}\n.ends",
            i + 1,
            i + 1
        )
        .unwrap();
    }
    writeln!(definitions, ".subckt t{} n", usize::BITS).unwrap();
    let root = "V1 a 0 1\nXtop a t0\n";
    let h = parse(&format!("{root}{definitions}.ends")).unwrap();
    assert_eq!(
        h.operating_point(Default::default()).unwrap().amperes.len(),
        1
    );
    let physical = format!("{definitions}Rload n 0 1\n.ends");
    let err = parse(&format!("{root}{physical}")).unwrap_err();
    assert_eq!(err.line, 2);
    assert!(err.message.contains("Xtop: expanded circuit size"));
    assert!(parse(&format!("V1 a 0 1\n{physical}")).is_ok());
}
