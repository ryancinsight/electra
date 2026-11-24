use super::{amps, volts};
use electra_circuit::{MnaError, parse};

#[test]
fn negative_feedback_sets_closed_loop_gain_and_delivers_load_current() {
    // Vout=A*(Vin-Vout/2), including the current through the feedback divider.
    let c = parse("Vin in 0 0.25\nEop out 0 in fb 100000\nRu out fb 1k\nRd fb 0 1k\nRl out 0 2k")
        .unwrap();
    let p = c.operating_point_mna(Default::default()).unwrap();
    let v = 0.25 * 100000. / 50001.;
    volts(&p.probes, "out", v);
    amps(&p.probes, "Eop", -v / 1000.);
}
#[test]
fn sensed_currents_can_be_forward_referenced_dependent_voltage_branches() {
    let c=parse("Fcopy 0 z hNext 2\nHnext y 0 esource -5\nEsource x 0 a 0 3\nVin a 0 2\nRx x 0 2\nRy y 0 5\nRz z 0 1").unwrap();
    let p = c.operating_point_mna(Default::default()).unwrap();
    volts(&p.probes, "x", 6.);
    amps(&p.probes, "Esource", -3.);
    volts(&p.probes, "y", 15.);
    amps(&p.probes, "Hnext", -3.);
    volts(&p.probes, "z", -6.);
}
#[test]
fn a_self_sensing_transconductance_can_establish_a_unique_potential() {
    let c = parse("Gload n 0 n 0 2m\nI1 0 n 6m").unwrap();
    let p = c.operating_point_mna(Default::default()).unwrap();
    volts(&p.probes, "n", 3.);
    amps(&p.probes, "Gload", 0.006);
}
#[test]
fn singular_feedback_and_unconstrained_control_inputs_reject_zero_rhs() {
    for deck in [
        "Eunity n 0 n 0 1\nRload n 0 2",
        "Gcancel n 0 n 0 -0.25\nRload n 0 4",
        "E1 a 0 b 0 2\nE2 b 0 a 0 0.5",
        "Fcopy out 0 Vprobe -1\nVprobe out 0 0",
        "Eread out 0 float 0 1\nRout out 0 1",
        "Hfeedback n 0 Hfeedback -1\nRload n 0 1",
    ] {
        let c = parse(deck).unwrap();
        assert!(
            matches!(
                c.operating_point_mna(Default::default()),
                Err(MnaError::RankDeficient)
            ),
            "{deck}"
        );
    }
}
#[test]
fn coincident_sense_pins_and_zero_coefficients_return_zero_output() {
    let c = parse(
        "Vin in 0 1\nGi a 0 in in 1e300\nRa a 0 2\nEv b 0 in in -1e300\nHh c 0 Vin 0\nFf a 0 Vin 0",
    )
    .unwrap();
    let p = c.operating_point_mna(Default::default()).unwrap();
    for n in ["a", "b", "c"] {
        volts(&p.probes, n, 0.);
    }
    for s in ["Gi", "Ev", "Hh", "Ff"] {
        amps(&p.probes, s, 0.);
    }
}

#[test]
fn mixed_decks_preserve_independent_voltage_loop_diagnostics() {
    for (v, reason) in [(1, "redundant"), (2, "incompatible")] {
        let err = parse(&format!("Gread x 0 a 0 1m\nV1 a 0 1\nV2 a 0 {v}")).unwrap_err();
        assert_eq!(err.line, 3);
        assert!(err.message.contains(reason), "{err}");
    }
}

#[test]
fn floating_outputs_and_differential_sensing_preserve_both_terminal_signs() {
    let c = parse("Vrail rail 0 4\nVin sense rail 2\nRbias sense rail 2\nEamp e rail sense rail -2\nRe e rail 2\nGout g rail rail sense 0.5\nRg g rail 2\nFcopy f rail Vin -2\nRf f rail 2\nHout h rail Vin 3\nRh h rail 3").unwrap();
    let p = c.operating_point_mna(Default::default()).unwrap();
    for (name, value) in [
        ("rail", 4.),
        ("sense", 6.),
        ("e", 0.),
        ("g", 6.),
        ("f", 0.),
        ("h", 1.),
    ] {
        volts(&p.probes, name, value);
    }
    for (name, value) in [
        ("Vin", -1.),
        ("Eamp", 2.),
        ("Gout", -1.),
        ("Fcopy", 2.),
        ("Hout", 1.),
        ("Vrail", 0.),
    ] {
        amps(&p.probes, name, value);
    }
}

#[test]
fn a_self_controlled_voltage_branch_can_have_a_unique_current() {
    let c = parse("Hport n 0 Hport 2\nR1 n 0 1\nI1 0 n 3").unwrap();
    let p = c.operating_point_mna(Default::default()).unwrap();
    volts(&p.probes, "n", 2.);
    amps(&p.probes, "Hport", 1.);
}

#[test]
fn rank_check_accepts_a_grounded_ladder_in_forward_reverse_and_interleaved_order() {
    use std::fmt::Write;
    for stride in [1, 256, 73] {
        let mut deck = String::new();
        for k in 0..257 {
            let i = (k * stride + 256) % 257;
            if i == 0 {
                writeln!(deck, "R0 n0 0 1k").unwrap();
            } else {
                writeln!(deck, "R{i} n{i} n{} 1k", i - 1).unwrap();
            }
        }
        deck.push_str("Gload n0 0 n0 0 1m");
        let c = parse(&deck).unwrap();
        let p = c.operating_point_mna(Default::default()).unwrap();
        volts(&p.probes, "n256", 0.);
    }
}
