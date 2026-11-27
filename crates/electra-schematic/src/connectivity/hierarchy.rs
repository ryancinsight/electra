//! Sheet membership, ordered bus lanes and electrical junction incidence.
use super::{NetId, PinId, Schematic};
use std::collections::BTreeSet;

impl Schematic {
    /// Sheet containing a component. Root-level components have no enclosing sheet.
    pub fn sheet_of(&self, component: u64) -> Option<&str> {
        self.sheets
            .iter()
            .find(|(_, parts)| parts.contains(&component))
            .map(|(path, _)| path.as_str())
    }
    /// Components on the named sheet in persistent ID order.
    pub fn sheet_parts(&self, path: &str) -> impl Iterator<Item = u64> + '_ {
        self.sheets.get(path).into_iter().flatten().copied()
    }
    /// Define an ordered logical bus. Lane order is significant; lanes must be distinct.
    /// Nets may be declared before wiring, as for net labels.
    pub fn set_bus(
        &mut self,
        name: String,
        lanes: Vec<NetId>,
    ) -> Result<Option<Vec<NetId>>, &'static str> {
        if name.is_empty() || name.len() > usize::from(u16::MAX) {
            return Err("invalid bus name");
        }
        if lanes.is_empty() || lanes.len() > usize::from(u16::MAX) {
            return Err("invalid bus width");
        }
        let mut unique = BTreeSet::new();
        if !lanes.iter().all(|net| unique.insert(net)) {
            return Err("duplicate bus lane");
        }
        Ok(self.buses.insert(name, lanes))
    }
    /// Borrow bus lane identities in declared order.
    pub fn bus(&self, name: &str) -> Option<&[NetId]> {
        self.buses.get(name).map(Vec::as_slice)
    }
    /// Borrow each bus and its ordered lanes in name order.
    pub fn buses(&self) -> impl Iterator<Item = (&str, &[NetId])> {
        self.buses
            .iter()
            .map(|(name, lanes)| (name.as_str(), lanes.as_slice()))
    }
    /// Remove the named bus without changing any electrical connections.
    pub fn remove_bus(&mut self, name: &str) -> Option<Vec<NetId>> {
        self.buses.remove(name)
    }
    /// Traverse the terminals meeting at one electrical junction in pin order.
    pub fn junction(&self, net: NetId) -> impl Iterator<Item = PinId> + '_ {
        self.pins()
            .filter_map(move |(id, pin)| (pin.net == Some(net)).then_some(id))
    }
}
