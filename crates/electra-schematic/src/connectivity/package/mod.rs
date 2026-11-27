//! Compare placed pin assignments with a reusable symbol-to-package map.
use super::{PinId, Schematic};
use std::collections::BTreeMap;

/// Symbol-local pin numbers mapped to exact physical pad identifiers.
/// Pad names are case-sensitive and may be alphanumeric.
pub type PinMap = BTreeMap<u32, String>;

/// Package audit finding. Pad names borrow the caller's maps.
#[derive(Debug, PartialEq, Eq)]
pub enum PackageFinding<'a> {
    /// The package contract requires a terminal absent from this occurrence.
    MissingTerminal(PinId),
    /// An expected pad assignment is absent or names a different pad.
    WrongPad(PinId, &'a str, Option<&'a str>),
    /// A pad was assigned to a terminal outside the package contract.
    UnexpectedAssignment(PinId, &'a str),
}

impl Schematic {
    /// Audit one placed occurrence against its shared package definition.
    /// Every contract entry is required. Unassigned schematic-only terminals are allowed.
    /// Report contract findings first, then extraneous assignments, each in pin order.
    pub fn check_package<'a>(
        &self,
        instance: u64,
        contract: &'a PinMap,
        placed: &'a PinMap,
        mut report: impl FnMut(PackageFinding<'a>),
    ) {
        for (&pin, expected) in contract {
            let id = PinId { instance, pin };
            if !self.pins.contains_key(&id) {
                report(PackageFinding::MissingTerminal(id));
            }
            let actual = placed.get(&pin).map(String::as_str);
            if actual != Some(expected.as_str()) {
                report(PackageFinding::WrongPad(id, expected, actual));
            }
        }
        for (&pin, pad) in placed {
            if !contract.contains_key(&pin) {
                report(PackageFinding::UnexpectedAssignment(
                    PinId { instance, pin },
                    pad,
                ));
            }
        }
    }
}
