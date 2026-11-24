//! Analytical and failure contracts for dependent sources and their controller bindings.
use electra_circuit::DcResult;
#[path = "controlled/feedback.rs"]
mod feedback;
#[path = "controlled/hierarchy.rs"]
mod hierarchy;
use electra_circuit::{SourceValue, parse};
use std::process::Command;

fn check<D>(probes: &[(&str, aequitas::Quantity<f64, D>)], name: &str, want: f64, absolute: f64) {
    let value = probes
        .iter()
        .find(|(n, _)| *n == name)
        .unwrap()
        .1
        .into_base();
    assert!(
        (value - want).abs() <= absolute + 1e-9 * want.abs(),
        "{name}: expected {want}, got {value}"
    );
}
fn volts(r: &DcResult<'_>, name: &str, want: f64) {
    check(&r.volts, name, want, 1e-10)
}
fn amps(r: &DcResult<'_>, name: &str, want: f64) {
    check(&r.amperes, name, want, 1e-12)
}

fn gains(r: &DcResult<'_>, v: f64) {
    for (name, ratio) in [
        ("0", 0.),
        ("drive", 1.),
        ("sense", 0.),
        ("ev", -2.),
        ("gi", -2.),
        ("fi", 3.),
        ("hv", 2.),
    ] {
        volts(r, name, v * ratio);
    }
    let i = v / 1000.;
    for (name, ratio) in [
        ("Vin", -1.),
        ("Rsense", 1.),
        ("Vprobe", 1.),
        ("Eamp", 1.),
        ("Re", -1.),
        ("Gout", 1.),
        ("Rg", -1.),
        ("Fmirror", -3.),
        ("Rf", 3.),
        ("Hout", -2.),
        ("Rh", 2.),
    ] {
        amps(r, name, i * ratio);
    }
}
#[test]
fn all_four_gains_and_load_currents_follow_the_drive_through_zero() {
    let c = parse(include_str!("../../../examples/controlled-sources.cir")).unwrap();
    assert!(c.requires_mna());
    gains(
        &c.operating_point_mna(Default::default()).unwrap().probes,
        2.,
    );
    let mut sweep = c.dc_sweep_mna(Default::default()).unwrap();
    for v in [-2., -1., 0., 1., 2.] {
        let point = sweep.next_point().unwrap().unwrap();
        assert_eq!(
            point.excitation,
            SourceValue::Voltage(electra_circuit::ElectricPotential::from_base(v))
        );
        gains(&point.result.probes, v);
    }
    assert!(sweep.next_point().unwrap().is_none());
}
#[test]
fn cli_selects_mna_for_both_modes_and_rejects_forced_reduction() {
    let run = |opts: &[&str], mode: &str| {
        Command::new(env!("CARGO_BIN_EXE_electra"))
            .args([
                mode,
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../examples/controlled-sources.cir"
                ),
            ])
            .args(opts)
            .output()
            .unwrap()
    };
    for mode in ["dc", "op"] {
        let auto = run(&[], mode);
        let explicit = run(&["--solver", "mna"], mode);
        assert!(auto.status.success() && explicit.status.success());
        assert_eq!(auto.stdout, explicit.stdout);
        let forced = run(&["--solver", "reduced"], mode);
        assert!(!forced.status.success());
        assert!(
            String::from_utf8(forced.stderr)
                .unwrap()
                .contains("controlled sources require")
        );
    }
}
