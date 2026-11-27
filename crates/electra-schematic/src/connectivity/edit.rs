//! Reversible, all-or-nothing changes to pin connectivity.
use super::{NetId, PinId, Schematic};
use std::collections::{BTreeMap, btree_map::Entry};

/// Pin-specific reason a wiring command could not be constructed or applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RewireError {
    /// Two changes in the command target the same occurrence and pin.
    Duplicate(PinId),
    /// The schematic has no such pin.
    Missing(PinId),
    /// The live binding differs from the expected binding.
    Conflict(PinId),
}

/// One reversible edit. Storage scales with the number of targeted pins.
/// Editors can retain successful commands for undo and redo.
#[derive(Default)]
pub struct Rewire {
    changes: BTreeMap<PinId, (Option<NetId>, Option<NetId>)>,
}

impl Rewire {
    /// Stage a change from an expected binding to a new one. None means unwired.
    pub fn add(
        &mut self,
        id: PinId,
        before: Option<NetId>,
        after: Option<NetId>,
    ) -> Result<(), RewireError> {
        match self.changes.entry(id) {
            Entry::Occupied(_) => Err(RewireError::Duplicate(id)),
            Entry::Vacant(entry) => {
                entry.insert((before, after));
                Ok(())
            }
        }
    }

    /// Apply or redo the edit after validating every target.
    pub fn apply(&self, schematic: &mut Schematic) -> Result<(), RewireError> {
        self.update(schematic, false)
    }

    /// Restore prior bindings after validating every target's current binding.
    pub fn undo(&self, schematic: &mut Schematic) -> Result<(), RewireError> {
        self.update(schematic, true)
    }

    fn update(&self, schematic: &mut Schematic, reverse: bool) -> Result<(), RewireError> {
        for (&id, &(before, after)) in &self.changes {
            let pin = schematic.pins.get(&id).ok_or(RewireError::Missing(id))?;
            if pin.net != (if reverse { after } else { before }) {
                return Err(RewireError::Conflict(id));
            }
        }
        for (id, &(before, after)) in &self.changes {
            // Exclusive access and the validation pass guarantee that every pin exists.
            schematic.pins.get_mut(id).expect("validated pin").net =
                if reverse { before } else { after };
        }
        Ok(())
    }
}
