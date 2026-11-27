//! Replay wiring commands against saved and reopened connectivity.
use electra_schematic::{NetId, Pin, PinId, Rewire, RewireError, Role, Schematic};

fn snapshot(model: &Schematic) -> Vec<u8> {
    let mut bytes = Vec::new();
    model.write_connectivity(&mut bytes).unwrap();
    bytes
}

#[test]
fn edits_are_atomic_reversible_and_replayable_after_reopen() {
    let id = |instance| PinId { instance, pin: 1 };
    let net = Some(NetId(0));
    let mut model = Schematic::default();
    for instance in [1, 2] {
        model.insert(
            id(instance),
            Pin {
                required: true,
                role: Role::Passive,
                net: None,
            },
        );
    }
    let original = snapshot(&model);
    let mut edit = Rewire::default();
    for instance in [1, 2] {
        edit.add(id(instance), None, net).unwrap();
    }
    assert_eq!(
        edit.add(id(1), net, None),
        Err(RewireError::Duplicate(id(1)))
    );
    edit.apply(&mut model).unwrap();
    model.check(|f| panic!("{f:?}"));
    let wired = snapshot(&model);
    let mut reopened = Schematic::read_connectivity(wired.as_slice()).unwrap();
    reopened.pin_mut(id(2)).unwrap().net = None;
    let stale = snapshot(&reopened);
    assert_eq!(edit.undo(&mut reopened), Err(RewireError::Conflict(id(2))));
    assert_eq!(snapshot(&reopened), stale);
    reopened.pin_mut(id(2)).unwrap().net = net;
    edit.undo(&mut reopened).unwrap();
    assert_eq!(snapshot(&reopened), original);
    edit.apply(&mut reopened).unwrap();
    assert_eq!(snapshot(&reopened), wired);
    let mut invalid = Rewire::default();
    invalid.add(id(1), net, None).unwrap();
    invalid.add(id(3), None, net).unwrap();
    assert_eq!(
        invalid.apply(&mut reopened),
        Err(RewireError::Missing(id(3)))
    );
    assert_eq!(snapshot(&reopened), wired);
}
