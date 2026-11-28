//! Check exact clearance boundaries, layer isolation and ordinary via connectivity.
use electra_board::{
    board::NetId,
    geom::{Nm, Point},
    rules::contact::{Conductor, Finding, check},
};
use std::collections::BTreeSet;
fn pad(net: u32, x: i64, layer: u16) -> Conductor {
    let p = Point::new(Nm(x), Nm(0));
    Conductor {
        net: NetId(net),
        axis: [p, p],
        width: Nm(100),
        layers: [layer, layer],
        terminal: true,
        drill: None,
    }
}
fn findings(items: &[Conductor]) -> Vec<Finding> {
    let mut result = Vec::new();
    check(items, Nm(20), Nm(10), Nm(5), &BTreeSet::new(), |f| {
        result.push(f)
    })
    .unwrap();
    result
}
#[test]
fn gap_equality_is_legal_and_touch_is_a_short_only_on_common_layers() {
    assert!(findings(&[pad(1, 0, 0), pad(2, 120, 0)]).is_empty());
    assert_eq!(
        findings(&[pad(1, 0, 0), pad(2, 119, 0)]),
        [Finding::Clearance(0, 1)]
    );
    assert_eq!(
        findings(&[pad(1, 0, 0), pad(2, 100, 0)]),
        [Finding::Short(0, 1)]
    );
    assert!(findings(&[pad(1, 0, 0), pad(2, 0, 1)]).is_empty());
}
#[test]
fn buried_and_through_vias_connect_only_layers_within_their_span() {
    let ends = [pad(3, 0, 0), pad(3, 0, 3)];
    assert_eq!(findings(&ends), [Finding::Open(NetId(3))]);
    let mut via = Conductor {
        layers: [1, 2],
        terminal: false,
        drill: Some(Nm(90)),
        ..ends[0]
    };
    assert_eq!(
        findings(&[ends[0], ends[1], via]),
        [Finding::Open(NetId(3))]
    );
    via.layers = [0, 3];
    assert!(findings(&[ends[0], ends[1], via]).is_empty());
}
