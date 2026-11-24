use electra_circuit::{ElectricPotential, parse};

#[test]
fn zero_diagonal_ideal_source_alone_and_loaded_both_polarities() {
    for (ends, value) in [("a 0", 5.), ("0 a", -5.)] {
        let alone = parse(&format!("Vs {ends} 5")).unwrap();
        let p = alone.operating_point_mna(Default::default()).unwrap();
        assert_eq!(p.probes.volts[1].1.into_base(), value);
        assert_eq!(p.probes.amperes[0].1.into_base(), 0.);
        assert_eq!(p.residual_volts, ElectricPotential::default());
        let loaded = parse(&format!("Vs {ends} 5\nR a 0 2")).unwrap();
        let p = loaded.operating_point_mna(Default::default()).unwrap();
        assert!((p.probes.amperes[0].1.into_base() + 2.5).abs() < 1e-14);
        assert!((p.probes.amperes[1].1.into_base() - value / 2.).abs() < 1e-14);
    }
}

#[test]
fn scaling_resolves_resistance_decades_without_mixing_voltage_and_current_units() {
    for exp in [-250, 0, 250] {
        let c = parse(&format!(
            "V1 rail 0 3\nR1 rail mid 1e{exp}\nVs mid tap 0\nR2 tap 0 2e{exp}"
        ))
        .unwrap();
        let p = c.operating_point_mna(Default::default()).unwrap();
        assert!((p.probes.volts[3].1.into_base() - 2.).abs() < 1e-12);
        let amps = 10f64.powi(-exp);
        for ((_, i), sign) in p.probes.amperes.iter().zip([-1., 1., 1., 1.]) {
            assert!((i.into_base() / amps - sign).abs() < 1e-13);
        }
        assert!(p.residual_volts.into_base() < 1e-12);
    }
}

#[test]
fn refinement_recovers_local_balance_with_a_large_unrelated_voltage() {
    let c = parse("Vhigh a 0 1e16\nVsmall b 0 0.125\nRload b 0 2").unwrap();
    let p = c.operating_point_mna(Default::default()).unwrap();
    assert!((p.probes.volts[2].1.into_base() - 0.125).abs() < 1e-12);
    assert!((p.probes.amperes[1].1.into_base() + 0.0625).abs() < 1e-12);
    assert!(p.probes.amperes[0].1.into_base().abs() < 1e-12);
}

#[test]
fn opposing_current_sources_preserve_small_bias_in_both_solvers() {
    for sources in [
        "Ia n 0 1e20\nIb n 0 1\nIc n 0 -1e20",
        "Ic n 0 -1e20\nIb n 0 1\nIa n 0 1e20",
    ] {
        let c = parse(&format!("{sources}\nR n 0 2")).unwrap();
        let a = c.operating_point(Default::default()).unwrap();
        let b = c.operating_point_mna(Default::default()).unwrap();
        assert_eq!(a.volts[1].1.into_base(), -2.);
        assert_eq!(b.probes.volts[1].1.into_base(), -2.);
        assert_eq!(b.probes.residual_amperes.into_base(), 0.);
    }
}

#[test]
fn floating_probe_retains_half_ampere_between_cancelling_large_sources() {
    let c = parse("Vs a b 0\nR1 a 0 1\nR2 b 0 1\nI1 b 0 1e20\nI2 b 0 1\nI3 b 0 -1e20").unwrap();
    for point in [
        c.operating_point(Default::default()).unwrap(),
        c.operating_point_mna(Default::default()).unwrap().probes,
    ] {
        assert!((point.volts[1].1.into_base() + 0.5).abs() < 1e-12);
        assert!((point.amperes[0].1.into_base() - 0.5).abs() < 1e-12);
    }
}
