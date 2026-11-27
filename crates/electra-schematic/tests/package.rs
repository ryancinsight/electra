//! Audit two occurrences of the same three-terminal package.
use electra_schematic::{PackageFinding::*, Pin, PinId, PinMap, Role, Schematic};

#[test]
fn swapped_pads_and_missing_supply_identify_only_the_faulty_occurrence() {
    let contract = PinMap::from([(1, "A1".into()), (2, "B1".into()), (3, "VDD".into())]);
    let mut placed = PinMap::from([(1, "B1".into()), (2, "A1".into()), (3, "VDD".into())]);
    let mut schematic = Schematic::default();
    let terminal = Pin {
        required: true,
        role: Role::Passive,
        net: None,
    };
    let fault = |pin| PinId { instance: 22, pin };
    for pin in 1..=3 {
        schematic.insert(PinId { instance: 11, pin }, terminal);
        if pin != 3 {
            schematic.insert(fault(pin), terminal);
        }
    }
    schematic.check_package(11, &contract, &contract, |f| panic!("{f:?}"));
    let mut findings = Vec::new();
    schematic.check_package(22, &contract, &placed, |f| findings.push(f));
    assert_eq!(
        findings,
        [
            WrongPad(fault(1), "A1", Some("B1")),
            WrongPad(fault(2), "B1", Some("A1")),
            MissingTerminal(fault(3))
        ]
    );
    placed.clone_from(&contract);
    schematic.insert(fault(3), terminal);
    schematic.check_package(22, &contract, &placed, |f| panic!("{f:?}"));
    placed.remove(&1);
    placed.insert(4, "01".into());
    let mut findings = Vec::new();
    schematic.check_package(22, &contract, &placed, |f| findings.push(f));
    assert_eq!(
        findings,
        [
            WrongPad(fault(1), "A1", None),
            UnexpectedAssignment(fault(4), "01")
        ]
    );
}
