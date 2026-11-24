use super::{amps, volts};
use electra_circuit::parse;

#[test]
fn repeated_cells_link_private_probes_and_sense_only_pins_by_instance() {
    let c=parse("Vin a 0 2\nVother b 0 -1\nX1 a out1 cell\nX2 b out2 cell k=-3x\n.subckt cell in out params: k=2x gm=1mS\nF1 0 out vS {k}\nRload out 0 1k\nG1 0 s in 0 {gm}\nVs s 0 0\n.ends").unwrap();
    let p = c.operating_point_mna(Default::default()).unwrap();
    volts(&p.probes, "out1", 4.);
    volts(&p.probes, "out2", 3.);
    amps(&p.probes, "X1/Vs", 0.002);
    amps(&p.probes, "X2/Vs", -0.001);
    amps(&p.probes, "X2/F1", 0.003);
}
#[test]
fn invalid_controllers_gains_and_sweep_targets_have_source_lines() {
    for (deck, line, msg) in [
        (
            "Fcopy a 0 missing 2\nRload a 0 1",
            1,
            "controlling branch missing",
        ),
        ("Rload a 0 1\nHout b 0 Rload 1", 2, "voltage source"),
        (
            "Vin n 0 1\n.subckt bad\nFcopy x 0 Vin 1\n.ends",
            3,
            "local voltage source",
        ),
        (
            ".param gain=2Ohm\nEout a 0 b 0 {gain}",
            2,
            "expected x, got Ohm",
        ),
        (".param gm=2ms\nGout a 0 b 0 {gm}", 2, "expected S, got s"),
        (
            ".param r=2mS\nHout a 0 Vin {r}\nVin b 0 0",
            2,
            "expected Ohm, got S",
        ),
        (
            "Vin a 0 1\nGout b 0 a 0 2\n.dc Gout 1 5 1",
            3,
            "independent",
        ),
        ("Vin a 0 1\nEbad x 0 a 3", 2, "expected"),
        ("Vin a 0 1\nFbad x 0 Vin 2 extra", 2, "trailing"),
    ] {
        let error = parse(deck).unwrap_err();
        assert_eq!(error.line, line, "{error}");
        assert!(error.message.contains(msg), "{error}");
    }
}
