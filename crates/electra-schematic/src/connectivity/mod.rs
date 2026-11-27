mod block;
mod hierarchy;
mod model;
pub use block::PlacementError;
pub use model::ModelBinding;
mod edit;
pub use edit::{Rewire, RewireError};
mod package;
pub use package::{PackageFinding, PinMap};
mod labels;
mod rules;
mod snapshot;
pub use rules::Diagnostic;
use std::collections::{BTreeMap, btree_map::Entry};

/// Persistent pin address. Instance IDs are unique across the project.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PinId {
    /// Placed instance, including its hierarchical occurrence.
    pub instance: u64,
    /// Pin identifier local to the symbol.
    pub pin: u32,
}

/// Project-wide electrical net identity shared by electrically joined pins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NetId(pub u64);

/// Electrical drive behavior relevant to the supported rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Passive or non-driving terminal.
    Passive,
    /// Push-pull output that requires exclusive drive.
    Output,
}

/// Electrical properties and optional net binding of a placed pin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pin {
    /// Whether this terminal needs an electrical peer.
    pub required: bool,
    /// Drive classification.
    pub role: Role,
    /// Assigned net, if wired.
    pub net: Option<NetId>,
}

/// Canonical connectivity indexed by persistent occurrence and pin IDs.
#[derive(Default)]
pub struct Schematic {
    pins: BTreeMap<PinId, Pin>,
    models: BTreeMap<u64, std::sync::Arc<ModelBinding>>,
    labels: BTreeMap<NetId, String>,
    sheets: BTreeMap<String, std::collections::BTreeSet<u64>>,
    buses: BTreeMap<String, Vec<NetId>>,
}

impl Schematic {
    /// Insert a pin. Duplicate addresses return false and preserve the existing pin.
    pub fn insert(&mut self, id: PinId, pin: Pin) -> bool {
        if let Entry::Vacant(entry) = self.pins.entry(id) {
            entry.insert(pin);
            true
        } else {
            false
        }
    }

    /// Borrow an existing pin to edit its properties or reconnect it.
    pub fn pin_mut(&mut self, id: PinId) -> Option<&mut Pin> {
        self.pins.get_mut(&id)
    }
}

impl Schematic {
    /// Iterate stable pin identities and properties in occurrence order without copying.
    pub fn pins(&self) -> impl Iterator<Item = (PinId, &Pin)> {
        self.pins.iter().map(|(&id, pin)| (id, pin))
    }
    /// Iterate named nets, including named ports that have no terminals yet.
    pub fn labels(&self) -> impl Iterator<Item = (NetId, &str)> {
        self.labels.iter().map(|(&id, label)| (id, label.as_str()))
    }
}
