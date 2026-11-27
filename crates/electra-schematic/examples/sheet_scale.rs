//! Time placement of an already-authored 1,024-sheet project with eight parts per sheet.
//! The outer timer excludes fixture construction, verification and destruction.
use electra_schematic::{Pin, PinId, Role, Schematic};
use std::{collections::BTreeMap, time::Instant};

fn main() {
    let mut leaf = Schematic::default();
    for instance in 0..8 {
        leaf.insert(
            PinId { instance, pin: 1 },
            Pin {
                required: false,
                role: Role::Passive,
                net: None,
            },
        );
    }
    let mut template = Schematic::default();
    for sheet in 0..1024 {
        let parts = (0..8).map(|part| (part, 8 * sheet + part)).collect();
        template
            .place_block(&leaf, &format!("S{sheet}"), &parts, &BTreeMap::new())
            .unwrap();
    }
    let parts = (0..8192).map(|id| (id, id)).collect();
    let mut result = Schematic::default();
    let start = Instant::now();
    result
        .place_block(&template, "rack", &parts, &BTreeMap::new())
        .unwrap();
    let micros = start.elapsed().as_micros();
    assert_eq!(result.pins().count(), 8192);
    assert_eq!(result.sheet_parts("rack/S1023").count(), 8);
    println!("{micros} us");
}
