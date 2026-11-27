//! Verify occurrence-specific probes and the native label table.
use electra_schematic::{NetId, Pin, PinId, Role, Schematic};

#[test]
fn equal_names_preserve_distinct_nets_across_reopen() {
    let mut model = Schematic::default();
    for instance in [2, 1] {
        let net = NetId(instance);
        model.insert(
            PinId { instance, pin: 1 },
            Pin {
                required: true,
                role: Role::Passive,
                net: Some(net),
            },
        );
        model.set_label(net, "入力".into()).unwrap();
    }
    let mut bytes = Vec::new();
    model.write_connectivity(&mut bytes).unwrap();
    assert_eq!(bytes[4], 2);
    let mut model = Schematic::read_connectivity(bytes.as_slice()).unwrap();
    for instance in [1, 2] {
        assert_eq!(
            model.probe_pin(PinId { instance, pin: 1 }),
            Some((NetId(instance), Some("入力")))
        );
    }
    assert!(model.set_label(NetId(1), String::new()).is_err());
    assert_eq!(model.remove_label(NetId(1)).as_deref(), Some("入力"));
    assert_eq!(
        model.probe_pin(PinId {
            instance: 1,
            pin: 1
        }),
        Some((NetId(1), None))
    );
    assert_eq!(model.label(NetId(2)), Some("入力"));
    for end in 0..bytes.len() {
        assert!(Schematic::read_connectivity(&bytes[..end]).is_err());
    }
    *bytes.last_mut().unwrap() = 0xff;
    assert!(Schematic::read_connectivity(bytes.as_slice()).is_err());
}
