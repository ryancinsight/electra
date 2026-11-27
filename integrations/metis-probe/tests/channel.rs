//! Two-channel block placement, pin-fault inspection and post-reopen input replay.
use electra_ac::{
    Capacitance, ElectricConductance, ElectricPotential, ElectricalImpedance, Frequency, RcLadder,
    Stage,
};
use electra_iris_ac::{ShuntTerminal, probe::ProbeTable, visit_rc_probe};
use electra_metis_probe::PinList;
use electra_schematic::{
    NetId, PackageFinding, Pin, PinId, PinMap, PlacementError, Role, Schematic,
};
use metis_core::input::Key;
use std::collections::BTreeMap;

fn id(instance: u64, pin: u32) -> PinId {
    PinId { instance, pin }
}
fn save(model: &Schematic) -> Vec<u8> {
    let mut data = Vec::new();
    model.write_connectivity(&mut data).unwrap();
    data
}

#[test]
fn two_channel_block_fault_audit_and_metis_iris_replay() {
    // One channel comprises a three-terminal packaged device and a load.
    let mut block = Schematic::default();
    for (part, pin, net) in [(1, 1, 1), (1, 2, 2), (1, 3, 3), (2, 1, 2), (2, 2, 0)] {
        block.insert(
            id(part, pin),
            Pin {
                required: true,
                role: Role::Passive,
                net: Some(NetId(net)),
            },
        );
    }
    block.set_label(NetId(2), "OUT".into()).unwrap();
    let bindings = |base: u64| BTreeMap::from([(1, base + 1), (2, base + 2)]);
    let nets = |output| {
        BTreeMap::from([
            (NetId(0), NetId(0)),
            (NetId(1), NetId(1)),
            (NetId(2), NetId(output)),
            (NetId(3), NetId(3)),
        ])
    };
    let mut project = Schematic::default();
    project
        .place_block(&block, "channel/A", &bindings(10), &nets(12))
        .unwrap();
    let original = save(&project);
    assert_eq!(
        project.place_block(&block, "channel/B", &bindings(20), &nets(12)),
        Err(PlacementError::OccupiedNet(NetId(12)))
    );
    assert_eq!(save(&project), original);
    project
        .place_block(&block, "channel/B", &bindings(20), &nets(22))
        .unwrap();
    let supply = id(21, 3);
    let removed = project.remove_pin(supply).unwrap();
    let contract = PinMap::from([(1, "A1".into()), (2, "B1".into()), (3, "VDD".into())]);
    let swapped = PinMap::from([(1, "B1".into()), (2, "A1".into()), (3, "VDD".into())]);
    let mut findings = Vec::new();
    project.check_package(11, &contract, &contract, |f| panic!("{f:?}"));
    project.check_package(21, &contract, &swapped, |f| findings.push(f));
    assert_eq!(
        findings,
        [
            PackageFinding::WrongPad(id(21, 1), "A1", Some("B1")),
            PackageFinding::WrongPad(id(21, 2), "B1", Some("A1")),
            PackageFinding::MissingTerminal(supply)
        ]
    );
    project.insert(supply, removed);
    project.check_package(21, &contract, &contract, |f| panic!("{f:?}"));
    let reopened = Schematic::read_connectivity(save(&project).as_slice()).unwrap();
    let rows = [id(12, 1), id(22, 1)];
    let mut list = PinList::new(&rows, 160., 24.).unwrap();
    let freq = [Frequency::from_base(1000.)];
    let solve = |drive| {
        let cells = [Stage::new(
            ElectricalImpedance::from_base(1000.),
            Capacitance::from_base(0.),
            ElectricConductance::from_base(0.001),
        )
        .unwrap()];
        let mut result = None;
        visit_rc_probe(
            RcLadder::new(&cells).unwrap(),
            &freq,
            ElectricPotential::from_base(drive),
            ShuntTerminal::Output,
            |_, v, i| result = Some(([v], [i])),
        )
        .unwrap();
        result.unwrap()
    };
    let (va, ia) = solve(2.);
    let (vb, ib) = solve(4.);
    let table = ProbeTable {
        frequencies: &freq,
        voltages: BTreeMap::from([(NetId(12), va.as_slice()), (NetId(22), vb.as_slice())]),
        currents: BTreeMap::from([(rows[0], ia.as_slice()), (rows[1], ib.as_slice())]),
    };
    let inspect = |pin, label, net, volts, amps| {
        let traces = table.select(&reopened, pin).unwrap().unwrap();
        assert_eq!(traces.net, net);
        let v = traces.voltage.unwrap();
        let i = traces.current.unwrap();
        assert_eq!((v.name(), i.name()), (label, label));
        assert_eq!(v.samples()[0].into_base(), volts);
        assert_eq!(i.samples()[0].into_base(), amps);
    };
    list.press([12., 36.]).unwrap();
    list.release([12., 36.]).unwrap();
    inspect(
        list.selected().unwrap(),
        "channel/B/OUT",
        NetId(22),
        2.,
        0.002,
    );
    list.key(Key::from_windows_virtual_key(0x26).unwrap());
    inspect(
        list.selected().unwrap(),
        "channel/A/OUT",
        NetId(12),
        1.,
        0.001,
    );
}
