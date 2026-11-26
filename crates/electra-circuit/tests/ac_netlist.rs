//! Public AC deck and command-line qualification.
use electra_ac::{Complex64, Frequency};
use electra_circuit::parse_ac;
use std::{f64::consts::TAU, path::Path, process::Command};

#[test]
fn exported_bridge_equations_agree_with_analytical_solution() {
    let output = Command::new(env!("CARGO_BIN_EXE_electra"))
        .arg("ac-matrix")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/ac-bridge.cir"))
        .arg(TAU.recip().to_string())
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output.stderr);
    let csv = String::from_utf8(output.stdout).unwrap();
    assert!(csv.contains("free,2,,\"a\",,"));
    assert!(csv.contains("free,3,,\"b\",,"));
    let den = Complex64::new(6., 5.);
    let v = [Complex64::new(3., 2.) / den, Complex64::new(2., 2.) / den];
    let mut residual = [Complex64::default(); 2];
    let mut terms = 0;
    for row in csv.lines().skip(1) {
        let f: Vec<_> = row.split(',').collect();
        if !matches!(f[0], "Y" | "rhs") {
            continue;
        }
        terms += 1;
        let i = f[1].parse::<usize>().unwrap() - 2;
        let z = Complex64::new(f[4].parse().unwrap(), f[5].parse().unwrap());
        match f[0] {
            "Y" => residual[i] += z * v[f[2].parse::<usize>().unwrap() - 2],
            _ => residual[i] -= z,
        }
    }
    assert_eq!(terms, 6);
    assert!(residual.iter().all(|z| z.re.hypot(z.im) < 1e-15));
}

#[test]
fn deck_reuses_nodes_across_frequencies_and_handles_reversed_source() {
    let deck = parse_ac("L1 Out 0 1m\nR1 OUT In 1k\nv1 0 IN ac 2 450").unwrap();
    assert_eq!(deck.nodes(), ["0", "Out", "In"]);
    assert_eq!(
        deck.boundary()[2].unwrap().into_base(),
        Complex64::new(0., -2.)
    );
    let stamp = |hz| {
        let mut y = Complex64::default();
        let mut rhs = Complex64::default();
        deck.stamp(
            Frequency::from_base(hz),
            |i, j, z| {
                assert_eq!((i, j), (1, 1));
                y += z.into_base();
            },
            |i, z| {
                assert_eq!(i, 1);
                rhs += z.into_base();
            },
        )?;
        Ok::<_, electra_circuit::ParseError>((y, rhs))
    };
    assert_eq!(stamp(0.).unwrap_err().line, 1);
    for hz in [100., 2000.] {
        let (y, rhs) = stamp(hz).unwrap();
        assert_eq!(y.re, 0.001);
        assert!((y.im + 1000. / (TAU * hz)).abs() < 1e-14);
        assert_eq!(rhs, Complex64::new(0., -0.002));
    }
}

#[test]
fn invalid_cards_report_the_offending_line() {
    for card in [
        "r1 other 0 2",
        "V1 a out AC 1",
        "V1 0 0 AC 1",
        "V1 out 0 AC -1",
        "V1 out 0 2",
        "V1 out 0 AC 1 junk",
        "R2 n 0 1kohm",
        "C1 n 0 -1",
        "L1 a b 0",
        ".ac dec 10 1 100",
        "I1 out 0 AC 1",
        ".end extra",
        "R2 out",
    ] {
        assert_eq!(
            parse_ac(&format!("R1 out 0 1\n{card}")).unwrap_err().line,
            2,
            "{card}"
        );
    }
    assert_eq!(
        parse_ac("V1 a 0 AC 1\nR2 a 0 1\nV2 0 A AC 2")
            .unwrap_err()
            .line,
        3
    );
    assert_eq!(
        parse_ac("R1 n 0 1\n.end\n* comment\nC1 n 0 1")
            .unwrap_err()
            .line,
        4
    );
}

#[test]
fn csv_retains_small_source_terms_across_cancelling_drives() {
    let path =
        std::env::temp_dir().join(format!("electra-cancellation-{}.cir", std::process::id()));
    for branches in [
        "R1 p out 1\nR2 q out 1\nR3 n out 1",
        "R3 n out 1\nR1 p out 1\nR2 q out 1",
    ] {
        std::fs::write(
            &path,
            format!("Vp p 0 AC 1e16\nVq q 0 AC 1\nVn n 0 AC 1e16 180\n{branches}"),
        )
        .unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_electra"))
            .arg("ac-matrix")
            .arg(&path)
            .arg("1")
            .output()
            .unwrap();
        assert!(output.status.success());
        let csv = String::from_utf8(output.stdout).unwrap();
        assert!(csv.lines().any(|s| s == "rhs,4,,,1,0"), "{csv}");
    }
    std::fs::remove_file(path).unwrap();
}
