//! Nested sheet, bus ordering and junction identity survive reopen.
use electra_schematic::{NetId, Pin, PinId, Role, Schematic};
use std::collections::BTreeMap;
#[test]
fn place_nested_sheet_preserves_lane_order_and_membership() {
    let mut leaf = Schematic::default();
    for pin in [1, 2, 3] {
        leaf.insert(
            PinId { instance: 1, pin },
            Pin {
                net: Some(NetId(1)),
                role: Role::Passive,
                required: true,
            },
        );
    }
    leaf.set_bus("data".into(), vec![NetId(2), NetId(1)])
        .unwrap();
    let parts = BTreeMap::from([(1, 1)]);
    let nets = BTreeMap::from([(NetId(1), NetId(1)), (NetId(2), NetId(2))]);
    let mut middle = Schematic::default();
    middle.place_block(&leaf, "inner", &parts, &nets).unwrap();
    let mut outer = Schematic::default();
    outer.place_block(&middle, "outer", &parts, &nets).unwrap();
    let mut bytes = Vec::new();
    outer.write_connectivity(&mut bytes).unwrap();
    let doc = Schematic::read_connectivity(bytes.as_slice()).unwrap();
    assert_eq!(doc.sheet_of(1), Some("outer/inner"));
    assert_eq!(doc.sheet_parts("outer/inner").collect::<Vec<_>>(), [1]);
    assert_eq!(doc.bus("outer/inner/data").unwrap(), [NetId(2), NetId(1)]);
    assert_eq!(doc.junction(NetId(1)).count(), 3);
}

#[test]
fn invalid_bus_mapping_leaves_placed_project_and_block_unchanged() {
    let mut block = Schematic::default();
    block
        .set_bus("pair".into(), vec![NetId(11), NetId(12)])
        .unwrap();
    let mut project = Schematic::default();
    project.set_label(NetId(0), "GND".into()).unwrap();
    let snapshot = |model: &Schematic| {
        let mut bytes = Vec::new();
        model.write_connectivity(&mut bytes).unwrap();
        bytes
    };
    let original = snapshot(&project);
    let parts = BTreeMap::new();
    for mapping in [
        BTreeMap::from([(NetId(11), NetId(7))]),
        BTreeMap::from([(NetId(11), NetId(7)), (NetId(12), NetId(7))]),
    ] {
        assert!(
            project
                .place_block(&block, "sub", &parts, &mapping)
                .is_err()
        );
        assert_eq!(snapshot(&project), original);
    }
    assert_eq!(block.bus("pair").unwrap(), [NetId(11), NetId(12)]);
}
