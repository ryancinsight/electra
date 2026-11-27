//! Regression fixture for occurrence-aware electrical rule checks.
use electra_schematic::{Diagnostic, NetId, Pin, PinId, Role, Schematic};

#[test]
fn reused_symbol_occurrences_report_exact_faults_and_clear_after_rewire() {
    let id = |instance| PinId { instance, pin: 1 };
    let a = NetId(7);
    let b = NetId(9);
    let mut circuit = Schematic::default();
    for (instance, role, net) in [
        (3, Role::Passive, None),
        (2, Role::Output, Some(a)),
        (1, Role::Output, Some(a)),
        (4, Role::Passive, Some(b)),
    ] {
        assert!(circuit.insert(
            id(instance),
            Pin {
                required: true,
                role,
                net
            }
        ));
    }
    assert!(!circuit.insert(
        id(1),
        Pin {
            required: false,
            role: Role::Passive,
            net: None
        }
    ));
    let mut findings = Vec::new();
    circuit.check(|item| findings.push(item));
    assert_eq!(
        findings,
        [
            Diagnostic::OutputConflict(a, id(1), id(2)),
            Diagnostic::Unconnected(id(3)),
            Diagnostic::Unconnected(id(4)),
        ]
    );
    circuit.pin_mut(id(2)).unwrap().net = Some(b);
    circuit.pin_mut(id(3)).unwrap().net = Some(a);
    circuit.check(|item| panic!("unexpected {item:?}"));
}
