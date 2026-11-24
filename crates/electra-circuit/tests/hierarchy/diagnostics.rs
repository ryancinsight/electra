use electra_circuit::parse;

fn error(deck: &str, line: usize, fragments: &[&str]) {
    let e = parse(deck).unwrap_err();
    assert_eq!(e.line, line, "{e}");
    for fragment in fragments {
        assert!(e.message.contains(fragment), "{e} missing '{fragment}'");
    }
}

#[test]
fn instance_dimension_arity_and_visibility_diagnostics() {
    let model =
        ".subckt cell a params: r=2kOhm\nRload a 0 {r}\n.param local=1Ohm\n.ends\n.param t=5ms\n";
    for (call, fragments) in [
        (
            "Xbad a cell r=1ms",
            vec!["Xbad", "cell.r", "expected Ohm, got s"],
        ),
        (
            "Xbad a cell r={t}",
            vec!["Xbad", "cell.r", "parameter t", "expected Ohm, got s"],
        ),
        ("Xbad a cell R=1 r=2", vec!["Xbad", "duplicate override"]),
        (
            "Xbad a cell local=1",
            vec!["Xbad", "no formal parameter local"],
        ),
        (
            "Xbad a cell r={missing}",
            vec!["Xbad", "unknown local parameter missing"],
        ),
        ("Xbad a b cell", vec!["Xbad", "expects 1 pins, got 2"]),
        ("Xbad a absent", vec!["Xbad", "unknown subcircuit absent"]),
    ] {
        error(&format!("{model}{call}"), 6, &fragments);
    }
    error(
        ".param shared=1Ohm\n.subckt cell a\nR1 a 0 {shared}\n.ends",
        3,
        &["unknown local parameter shared"],
    );
    error(".param r=5\nV1 n 0 1", 1, &["needs a unit"]);
    error(".param t=1mS\nV1 n 0 1", 1, &["needs a unit"]);
    error(".param r=💡\nV1 n 0 1", 1, &["needs a unit"]);
}

#[test]
fn case_folded_collisions_and_reserved_names_have_source_lines() {
    for (s, line, msg) in [
        (".subckt c p P\n.ends", 1, "formal pin"),
        (".subckt c 0\n.ends", 1, "formal pin"),
        (
            ".subckt c a params: x=1V X=2V\n.ends",
            1,
            "duplicate parameter",
        ),
        (
            ".subckt cell a\nR1 a 0 1\nr1 a 0 2\n.ends",
            3,
            "duplicate element",
        ),
        (
            ".subckt cell\n.ends\nXfoo cell\nxFOO cell",
            4,
            "duplicate element or instance",
        ),
        (
            ".subckt cell\n.ends\n.subckt CELL\n.ends",
            3,
            "duplicate subcircuit",
        ),
        (
            ".param r=1Ohm\nR1 private/name 0 {r}",
            2,
            "invalid hierarchy identifier",
        ),
        (
            ".subckt c a\n.subckt d b\n.ends\n.ends",
            2,
            "nested .subckt",
        ),
        (".subckt c a\n.ends d", 2, "expected .ends c"),
        (".subckt c a\nR1 a 0 1", 1, "missing .ends"),
        (".ends", 1, "outside a subcircuit"),
        (".subckt c a\n.op\n.ends", 2, "requires root scope"),
        (".subckt c a\n.include file\n.ends", 2, "file scope"),
        (".include file", 1, "parse_file"),
    ] {
        error(s, line, &[msg]);
    }
    assert!(parse("R1 private/name 0 1").is_ok());
}

#[test]
fn recursive_model_graphs_fail_even_if_uninstantiated() {
    error(
        "V1 n 0 1\n.subckt a\nXb b\n.ends\n.subckt b\nXa a\n.ends",
        6,
        &["recursive subcircuit", "a -> b -> a"],
    );
    error(
        "Xtop loop\n.subckt loop\nXagain loop\n.ends",
        3,
        &["Xagain", "recursive subcircuit"],
    );
}
