//! Replay the public RC CLI and read the same output through a reopened schematic.
use electra_ac::{
    Capacitance, ElectricConductance, ElectricPotential, ElectricalImpedance, Frequency, LogSweep,
    RcLadder, Stage,
};
use electra_iris_ac::{RcOutputError, probe::ProbeTable, rc_output_rms};
use electra_schematic::{NetId, Pin, PinId, Role, Schematic};
use std::{collections::BTreeMap, process::Command};

#[test]
fn reopened_voltage_probe_matches_every_rc_cli_sample() {
    let cli = Command::new(env!("CARGO"))
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .args([
            "run",
            "--quiet",
            "--locked",
            "-p",
            "electra-ac",
            "--bin",
            "electra-rc",
            "--",
            "10",
            "100000",
            "17",
            "1000",
            "1e-7",
            "0.001",
            "220",
            "2e-7",
            "0.002",
        ])
        .output()
        .unwrap();
    assert!(
        cli.status.success(),
        "{}",
        String::from_utf8_lossy(&cli.stderr)
    );
    let cell = |r, c, g| {
        Stage::new(
            ElectricalImpedance::from_base(r),
            Capacitance::from_base(c),
            ElectricConductance::from_base(g),
        )
        .unwrap()
    };
    let cells = [cell(1000., 1e-7, 0.001), cell(220., 2e-7, 0.002)];
    let model = RcLadder::new(&cells).unwrap();
    let grid: Vec<_> = LogSweep::new(Frequency::from_base(10.), Frequency::from_base(100000.), 17)
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let source = ElectricPotential::from_base(2.5);
    let voltage = rc_output_rms(model, &grid, source).unwrap();
    assert_eq!(
        rc_output_rms(model, &grid, ElectricPotential::from_base(-1.)),
        Err(RcOutputError::Source)
    );
    let net = NetId(17);
    let pin = PinId {
        instance: 4,
        pin: 2,
    };
    let mut schematic = Schematic::default();
    schematic.insert(
        pin,
        Pin {
            required: true,
            role: Role::Passive,
            net: Some(net),
        },
    );
    schematic.set_label(net, "filter/OUT".into()).unwrap();
    let mut bytes = Vec::new();
    schematic.write_connectivity(&mut bytes).unwrap();
    let reopened = Schematic::read_connectivity(bytes.as_slice()).unwrap();
    let table = ProbeTable {
        frequencies: &grid,
        voltages: BTreeMap::from([(net, voltage.as_slice())]),
        currents: BTreeMap::new(),
    };
    let view = table
        .select(&reopened, pin)
        .unwrap()
        .unwrap()
        .voltage
        .unwrap();
    assert_eq!(view.name(), "filter/OUT");
    assert!(std::ptr::eq(view.samples(), voltage.as_slice()));
    let csv = String::from_utf8(cli.stdout).unwrap();
    let mut rows = csv.lines();
    let headers: Vec<_> = rows.next().unwrap().split(',').collect();
    let column = |name| headers.iter().position(|s| *s == name).unwrap();
    let hz = column("frequency_Hz");
    let gain = column("gain_magnitude");
    for (f, v) in view.iter() {
        let row: Vec<_> = rows.next().unwrap().split(',').collect();
        assert_eq!(f.into_base(), row[hz].parse::<f64>().unwrap());
        assert_eq!(
            v.into_base(),
            source.into_base() * row[gain].parse::<f64>().unwrap()
        );
    }
    assert!(rows.next().is_none());
}
