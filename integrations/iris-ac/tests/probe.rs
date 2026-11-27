//! Cross-probe reopened connectivity against independently indexed result columns.
use electra_ac::{ElectricCurrent, ElectricPotential, Frequency};
use electra_iris_ac::probe::ProbeTable;
use electra_schematic::{NetId, Pin, PinId, Role, Schematic};
use std::collections::BTreeMap;

#[test]
fn shared_voltage_and_distinct_terminal_currents_survive_selection_after_reopen() {
    let pin = |instance| PinId { instance, pin: 1 };
    let net = NetId(1);
    let mut model = Schematic::default();
    for (instance, wired) in [
        (1, Some(net)),
        (2, Some(net)),
        (3, Some(NetId(2))),
        (4, None),
    ] {
        model.insert(
            pin(instance),
            Pin {
                required: true,
                role: Role::Passive,
                net: wired,
            },
        );
    }
    for id in [net, NetId(2)] {
        model.set_label(id, "OUT".into()).unwrap();
    }
    let mut saved = Vec::new();
    model.write_connectivity(&mut saved).unwrap();
    let model = Schematic::read_connectivity(saved.as_slice()).unwrap();
    let frequencies = [100., 1000.].map(Frequency::from_base);
    let volts = [2., 1.].map(ElectricPotential::from_base);
    let source = [-0.002, -0.001].map(ElectricCurrent::from_base);
    let load = [0.002, 0.001].map(ElectricCurrent::from_base);
    let mut table = ProbeTable {
        frequencies: &frequencies,
        voltages: BTreeMap::from([(net, volts.as_slice())]),
        currents: BTreeMap::from([(pin(1), source.as_slice()), (pin(2), load.as_slice())]),
    };
    for (instance, current) in [(1, &source), (2, &load)] {
        let traces = table.select(&model, pin(instance)).unwrap().unwrap();
        let v = traces.voltage.unwrap();
        let i = traces.current.unwrap();
        assert_eq!((v.name(), i.name()), ("OUT", "OUT"));
        assert!(std::ptr::eq(v.samples(), volts.as_slice()));
        assert!(std::ptr::eq(i.samples(), current.as_slice()));
        assert_eq!(i.iter().nth(1), Some((&frequencies[1], &current[1])));
    }
    let absent = table.select(&model, pin(3)).unwrap().unwrap();
    assert!(absent.voltage.is_none() && absent.current.is_none());
    assert!(table.select(&model, pin(4)).unwrap().is_none());
    table.currents.insert(pin(2), &load[..1]);
    assert!(table.select(&model, pin(2)).is_err());
}
