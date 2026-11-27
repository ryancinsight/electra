//! Persistence and isolation of ordered model terminal maps.
use electra_schematic::{Pin, PinId, Role, Schematic};
use std::collections::BTreeMap;
#[test]
fn placed_models_share_definitions_and_reopen_in_terminal_order() {
    let mut block = Schematic::default();
    for pin in [1, 2, 3] {
        block.insert(
            PinId { instance: 1, pin },
            Pin {
                required: true,
                role: Role::Passive,
                net: None,
            },
        );
    }
    block.bind_model(1, "device/μ".into(), vec![2, 1]).unwrap();
    assert!(block.bind_model(1, "bad".into(), vec![1, 1]).is_err());
    let mut legacy = Vec::new();
    block.write_connectivity(&mut legacy).unwrap();
    assert_eq!(legacy[4], 3);
    let old = Schematic::read_connectivity(legacy.as_slice()).unwrap();
    assert_eq!(old.model(1).unwrap().pins(), [2, 1]);
    assert_eq!(old.sheet_of(1), None);
    let mut project = Schematic::default();
    for id in [8, 9] {
        project
            .place_block(
                &block,
                "channel",
                &BTreeMap::from([(1, id)]),
                &BTreeMap::new(),
            )
            .unwrap();
    }
    assert!(std::ptr::eq(
        project.model(8).unwrap(),
        project.model(9).unwrap()
    ));
    let mut data = Vec::new();
    project.write_connectivity(&mut data).unwrap();
    assert_eq!(data[4], 4);
    for end in 0..data.len() {
        assert!(Schematic::read_connectivity(&data[..end]).is_err());
    }
    let mut restored = Schematic::read_connectivity(data.as_slice()).unwrap();
    assert_eq!(restored.model(8).unwrap().pins(), [2, 1]);
    assert_eq!(restored.model(9).unwrap().name(), "device/μ");
    let mut roundtrip = Vec::new();
    restored.write_connectivity(&mut roundtrip).unwrap();
    assert_eq!(roundtrip, data);
    restored.remove_pin(PinId {
        instance: 8,
        pin: 3,
    });
    assert!(restored.model(8).is_some());
    restored.remove_pin(PinId {
        instance: 8,
        pin: 2,
    });
    assert!(restored.model(8).is_none());
    assert!(restored.model(9).is_some());
    *data.last_mut().unwrap() = 255;
    assert!(Schematic::read_connectivity(data.as_slice()).is_err());
}
