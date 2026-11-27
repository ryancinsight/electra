//! Instantiate a reusable connectivity definition into project-wide identities.
use super::{NetId, PinId, Schematic};
use std::collections::{BTreeMap, BTreeSet};

/// A block placement failed before any project pins or labels were changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementError {
    /// Local component has no project instance binding.
    MissingInstance(u64),
    /// Project occurrence already exists or was assigned twice.
    OccupiedInstance(u64),
    /// Local connected or labeled net has no project binding.
    MissingNet(NetId),
    /// A private labeled net would overwrite another net's identity or label.
    OccupiedNet(NetId),
    /// The block path is empty or a qualified label exceeds the snapshot limit.
    InvalidPath,
    /// A placed bus would replace an existing named bus.
    OccupiedBus,
}

impl Schematic {
    /// Place a borrowed block with explicit component and net bindings. Labeled
    /// local nets are private to the block and acquire a path-qualified name.
    /// Unlabeled ports can bind shared project nets. All identities are validated
    /// before insertion; the project and the block need no full-size copies.
    pub fn place_block(
        &mut self,
        block: &Self,
        path: &str,
        instances: &BTreeMap<u64, u64>,
        nets: &BTreeMap<NetId, NetId>,
    ) -> Result<(), PlacementError> {
        if path.is_empty() || path.len() > usize::from(u16::MAX) {
            return Err(PlacementError::InvalidPath);
        }
        let mut assigned = BTreeSet::new();
        for &instance in instances.values() {
            let start = PinId { instance, pin: 0 };
            let end = PinId {
                instance,
                pin: u32::MAX,
            };
            if !assigned.insert(instance) || self.pins.range(start..=end).next().is_some() {
                return Err(PlacementError::OccupiedInstance(instance));
            }
        }
        for (id, pin) in &block.pins {
            if !instances.contains_key(&id.instance) {
                return Err(PlacementError::MissingInstance(id.instance));
            }
            if let Some(net) = pin.net
                && !nets.contains_key(&net)
            {
                return Err(PlacementError::MissingNet(net));
            }
        }
        let mut private = BTreeSet::new();
        for (net, label) in &block.labels {
            let &target = nets.get(net).ok_or(PlacementError::MissingNet(*net))?;
            if path.len().saturating_add(1).saturating_add(label.len()) > usize::from(u16::MAX) {
                return Err(PlacementError::InvalidPath);
            }
            if self.labels.contains_key(&target) || !private.insert(target) {
                return Err(PlacementError::OccupiedNet(target));
            }
        }
        for (local, target) in nets {
            if !block.labels.contains_key(local) && private.contains(target) {
                return Err(PlacementError::OccupiedNet(*target));
            }
        }
        if !private.is_empty() {
            for pin in self.pins.values() {
                if let Some(net) = pin.net
                    && private.contains(&net)
                {
                    return Err(PlacementError::OccupiedNet(net));
                }
            }
            if let Some(lane) = self
                .buses
                .values()
                .flatten()
                .find(|net| private.contains(net))
            {
                return Err(PlacementError::OccupiedNet(*lane));
            }
        }
        // Stage qualified metadata before inserting any project objects.
        let mut sheets: BTreeMap<String, BTreeSet<u64>> = BTreeMap::new();
        // Group nested parts once per sheet. A large block need not scan every
        // sheet or allocate a qualified path for each individual component.
        let mut root = BTreeSet::new();
        for id in block.pins.keys() {
            root.insert(instances[&id.instance]);
        }
        for (inner, parts) in &block.sheets {
            let scope = format!("{path}/{inner}");
            if scope.len() > usize::from(u16::MAX) {
                return Err(PlacementError::InvalidPath);
            }
            let mut members = BTreeSet::new();
            for local in parts {
                let placed = instances[local];
                root.remove(&placed);
                members.insert(placed);
            }
            sheets.insert(scope, members);
        }
        if !root.is_empty() {
            sheets.insert(path.to_owned(), root);
        }
        let mut buses = BTreeMap::new();
        for (local, lanes) in &block.buses {
            let name = format!("{path}/{local}");
            if name.len() > usize::from(u16::MAX) {
                return Err(PlacementError::InvalidPath);
            }
            if self.buses.contains_key(&name) {
                return Err(PlacementError::OccupiedBus);
            }
            let mut unique = BTreeSet::new();
            let mut placed = Vec::with_capacity(lanes.len());
            for lane in lanes {
                let net = *nets.get(lane).ok_or(PlacementError::MissingNet(*lane))?;
                if !unique.insert(net) {
                    return Err(PlacementError::OccupiedNet(net));
                }
                placed.push(net);
            }
            buses.insert(name, placed);
        }
        for (path, parts) in sheets {
            self.sheets.entry(path).or_default().extend(parts);
        }
        self.buses.append(&mut buses);
        for (id, pin) in &block.pins {
            let placed = PinId {
                instance: instances[&id.instance],
                pin: id.pin,
            };
            self.pins.insert(
                placed,
                super::Pin {
                    net: pin.net.map(|net| nets[&net]),
                    ..*pin
                },
            );
        }
        for (id, model) in &block.models {
            self.models.insert(instances[id], model.clone());
        }
        for (net, label) in &block.labels {
            self.labels.insert(nets[net], format!("{path}/{label}"));
        }
        Ok(())
    }

    /// Remove one placed terminal and return its properties. Removing a model
    /// terminal also clears its assignment to prevent dangling bindings.
    pub fn remove_pin(&mut self, id: PinId) -> Option<super::Pin> {
        if self
            .model(id.instance)
            .is_some_and(|m| m.pins().contains(&id.pin))
        {
            self.models.remove(&id.instance);
        }
        let removed = self.pins.remove(&id)?;
        let start = PinId {
            instance: id.instance,
            pin: 0,
        };
        let end = PinId {
            instance: id.instance,
            pin: u32::MAX,
        };
        if self.pins.range(start..=end).next().is_none() {
            self.sheets.retain(|_, parts| {
                parts.remove(&id.instance);
                !parts.is_empty()
            });
        }
        Some(removed)
    }
}
