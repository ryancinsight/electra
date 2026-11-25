//! Noise source laws, transferred spectra, and numerical limits.
use electra_ac::{
    AcError, Capacitance, ElectricConductance, ElectricalImpedance, Frequency, RcLadder, Stage,
    ThermodynamicTemperature,
    noise::{NoiseError, NoiseLadder, ThermalStage},
};

fn cell(r: f64, c: f64, g: f64) -> Stage {
    Stage::new(
        ElectricalImpedance::from_base(r),
        Capacitance::from_base(c),
        ElectricConductance::from_base(g),
    )
    .unwrap()
}
fn thermal(cell: Stage, tr: f64, tg: f64) -> Result<ThermalStage, NoiseError> {
    ThermalStage::new(
        cell,
        ThermodynamicTemperature::from_base(tr),
        ThermodynamicTemperature::from_base(tg),
    )
}
fn at(stages: &[ThermalStage], hz: f64) -> f64 {
    NoiseLadder::new(stages)
        .unwrap()
        .at(Frequency::from_base(hz))
        .unwrap()
        .into_base()
}
fn relative(value: f64, want: f64) {
    assert!((value / want - 1.).abs() < 1e-12, "{value:e} vs {want:e}");
}

#[test]
fn one_sided_resistor_and_capacitor_pole() {
    let open = thermal(cell(1000., 0., 0.), 300., 0.).unwrap();
    let rc = thermal(cell(1000., 1e-6, 0.), 300., 0.).unwrap();
    for hz in [0., 1., 1e9] {
        relative(at(&[open], hz), 1.656_778_8e-17);
    }
    let pole = 1000. / std::f64::consts::TAU;
    relative(at(&[rc], pole), 1.656_778_8e-17 / 2.);
    let cold = thermal(cell(1000., 1e-6, 0.001), 0., 0.).unwrap();
    assert_eq!(at(&[cold], pole), 0.);
}

#[test]
fn equilibrium_matches_real_port_impedance_and_local_sources_add_in_power() {
    let cells = [
        cell(22., 1e-9, 0.),
        cell(470., 0., 0.002),
        cell(150., 3e-8, 0.001),
    ];
    let uniform = cells.map(|s| thermal(s, 310., 310.).unwrap());
    let series = cells.map(|s| thermal(s, 310., 0.).unwrap());
    let shunt = cells.map(|s| thermal(s, 0., 310.).unwrap());
    let signal = RcLadder::new(&cells).unwrap();
    for hz in [0., 1e3, 1e5, 7e7] {
        let z = signal
            .at(Frequency::from_base(hz))
            .unwrap()
            .output_impedance
            .into_base();
        relative(at(&uniform, hz), 4. * 1.380649e-23 * 310. * z.re);
        relative(at(&series, hz) + at(&shunt, hz), at(&uniform, hz));
    }
}

#[test]
fn source_and_transfer_scale_preserve_representable_power() {
    let hot = thermal(cell(1e200, 1., 0.), 300., 0.).unwrap();
    relative(at(&[hot], 1. / std::f64::consts::TAU), 1.6567788e-220);
    let shunt = thermal(cell(1e200, 0., 1e-200), 0., 300.).unwrap();
    relative(at(&[shunt], 0.), 0.25 * 1.6567788e180);
    let series = thermal(cell(1e200, 0., 0.), 2e130, 0.).unwrap();
    let loaded = thermal(cell(1e200, 0., 1e-200), 2e130, 0.).unwrap();
    relative(at(&[series, loaded], 0.), (8. * 1.380649 / 9.) * 2e307);
    let frozen = thermal(cell(1e200, 0., 0.), 1e-320, 0.).unwrap();
    relative(
        at(&[frozen], 0.),
        (1e-320_f64 * 1e200) * (4. * 1.380649e-23),
    );
}

#[test]
fn local_noise_survives_underflow_of_end_to_end_signal_gain() {
    let cell = cell(1000., 1e-6, 0.001);
    let cells = [cell; 128];
    let stages = [thermal(cell, 300., 300.).unwrap(); 128];
    let hz = 1e8;
    assert!(
        RcLadder::new(&cells)
            .unwrap()
            .at(Frequency::from_base(hz))
            .is_err()
    );
    assert!(at(&stages, hz) > 0.);
}

#[test]
fn invalid_inputs_and_out_of_range_densities_return_errors() {
    let stage = cell(1000., 1e-9, 0.);
    for t in [-1., f64::INFINITY, f64::NAN] {
        assert_eq!(
            thermal(stage, t, 300.).unwrap_err(),
            NoiseError::Temperature
        );
        assert_eq!(
            thermal(stage, 300., t).unwrap_err(),
            NoiseError::Temperature
        );
    }
    assert_eq!(
        thermal(cell(1e300, 0., 0.), 1e300, 0.).unwrap_err(),
        NoiseError::SourceRange
    );
    assert_eq!(
        thermal(cell(1e-300, 0., 0.), 1e-300, 0.).unwrap_err(),
        NoiseError::SourceRange
    );
    assert_eq!(NoiseLadder::new(&[]).unwrap_err(), AcError::Empty);
    let stages = [thermal(stage, 300., 300.).unwrap()];
    let noise = NoiseLadder::new(&stages).unwrap();
    for f in [-1., f64::INFINITY, f64::NAN] {
        assert_eq!(
            noise.at(Frequency::from_base(f)).unwrap_err(),
            AcError::Frequency
        );
    }
    let tiny = [thermal(cell(1e200, 1e120, 0.), 300., 0.).unwrap()];
    assert_eq!(
        NoiseLadder::new(&tiny)
            .unwrap()
            .at(Frequency::from_base(1.))
            .unwrap_err(),
        AcError::DynamicRange { stage: 0 }
    );
}

#[test]
fn cli_rejects_invalid_stage_before_emitting_csv() {
    use std::process::Command;
    for (parts, message) in [
        (vec!["1000", "1e-9", "0", "310"], "SHUNT_K"),
        (vec!["1000", "1e-9", "0", "310", "-5"], "kelvin"),
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_electra-rc"))
            .args(["--noise", "10", "1000", "3"])
            .args(parts)
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        assert!(String::from_utf8(result.stderr).unwrap().contains(message));
    }
}
