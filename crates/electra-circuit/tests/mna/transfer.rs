use electra_circuit::{ElectricCurrent, LinearSweep, SourceValue, SweepRange, parse, parse_file};
use std::process::Command;

#[test]
fn sweep_reuses_buffers_preserves_orientation_and_nominal_source() {
    let c = parse("Ibig a b 1e25\nIbias 0 a 0.25\nR1 a 0 2\nR2 b 0 4").unwrap();
    let mut s = c
        .sweep_source_mna(
            "iBIG",
            SweepRange::Current(LinearSweep {
                start: ElectricCurrent::from_base(1.),
                stop: ElectricCurrent::from_base(-1.),
                step: ElectricCurrent::from_base(-0.5),
            }),
            Default::default(),
        )
        .unwrap();
    let mut buffers = None;
    for q in [1., 0.5, 0., -0.5, -1.] {
        let sample = s.next_point().unwrap().unwrap();
        assert_eq!(
            sample.excitation,
            SourceValue::Current(ElectricCurrent::from_base(q))
        );
        let p = &sample.result.probes;
        assert!((p.volts[1].1.into_base() - (0.25 - q) * 2.).abs() < 1e-12);
        assert!((p.volts[2].1.into_base() - 4. * q).abs() < 1e-12);
        assert_eq!(p.amperes[0].1.into_base(), q);
        let ptrs = (p.volts.as_ptr(), p.amperes.as_ptr());
        assert_eq!(*buffers.get_or_insert(ptrs), ptrs);
    }
    assert!(s.next_point().unwrap().is_none());
    let nominal = c.operating_point_mna(Default::default()).unwrap();
    assert_eq!(nominal.probes.amperes[0].1.into_base(), 1e25);
}

#[test]
fn qualified_sweeps_read_included_models_and_cli_rejects_misapplied_flags() {
    let file = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/hierarchical-divider.cir"
    );
    let c = parse_file(file).unwrap();
    let mut s = c.dc_sweep_mna(Default::default()).unwrap();
    let mut steps = 0;
    for rail in [0., 3., 6., 9.] {
        let p = s.next_point().unwrap().unwrap().result;
        let i = p
            .probes
            .amperes
            .iter()
            .find(|(name, _)| *name == "Xleft/Vsense")
            .unwrap()
            .1;
        assert!((i.into_base() - rail / 3000.).abs() < 1e-14);
        steps += p.iterations;
    }
    assert!(steps > 0);
    let out = Command::new(env!("CARGO_BIN_EXE_electra"))
        .args([
            "dc",
            file,
            "--solver",
            "mna",
            "--max-iterations",
            "50",
            "--atol-volts",
            "1e-12",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(text.matches("GMRES iterations)").count(), 4);
    assert_eq!(text.matches("I(Xleft/Vsense)").count(), 4);
    for wrong in [
        ["--solver", "bad"],
        ["--max-iterations", "2"],
        ["--atol-volts", "-1"],
    ] {
        assert!(
            !Command::new(env!("CARGO_BIN_EXE_electra"))
                .args(["op", file])
                .args(wrong)
                .output()
                .unwrap()
                .status
                .success()
        );
    }
}

#[test]
fn affine_seed_rechecks_each_source_voltage_across_a_zero_crossing() {
    let c = parse("Vrail a 0 1\nVf b a -2\nR1 b 0 4\n.dc Vf -3 3 0.25").unwrap();
    let mut sweep = c.dc_sweep_mna(Default::default()).unwrap();
    let mut skips = 0;
    for k in 0..=24 {
        let q = -3. + f64::from(k) / 4.;
        let p = sweep.next_point().unwrap().unwrap().result;
        assert!((p.probes.volts[2].1.into_base() - (1. + q)).abs() < 1e-12);
        assert!((p.probes.amperes[1].1.into_base() + (1. + q) / 4.).abs() < 1e-12);
        skips += usize::from(p.iterations == 0);
    }
    assert!(skips >= 20);
}

#[test]
fn swept_current_cancels_the_large_term_of_a_compensated_bias() {
    let c =
        parse("Ibig a 0 1e20\nIbias a 0 1\nIsweep 0 a 0\nR a 0 2\n.dc Isweep 1e20 1e20 1").unwrap();
    let mut g = c.dc_sweep(Default::default()).unwrap();
    let mut m = c.dc_sweep_mna(Default::default()).unwrap();
    assert_eq!(
        g.next_point().unwrap().unwrap().result.volts[1]
            .1
            .into_base(),
        -2.
    );
    assert_eq!(
        m.next_point().unwrap().unwrap().result.probes.volts[1]
            .1
            .into_base(),
        -2.
    );
}

#[test]
fn swept_rhs_is_normalized_only_after_applying_the_excitation() {
    // The fixed term by itself would normalize to 1e400. The first actual RHS is zero.
    let c = parse("Ifixed a 0 1e200\nIs 0 a 0\nR a 0 1e200\n.dc Is 1e200 1e200 1").unwrap();
    let mut m = c.dc_sweep_mna(Default::default()).unwrap();
    let p = m.next_point().unwrap().unwrap().result;
    assert_eq!(p.probes.volts[1].1.into_base(), 0.);
    assert_eq!(p.probes.amperes[1].1.into_base(), 1e200);
}
