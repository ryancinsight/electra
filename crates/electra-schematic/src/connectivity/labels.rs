//! Net display names are independent of connectivity and occurrence identity.
use super::{NetId, PinId, Schematic};

impl Schematic {
    /// Name a project net, returning its previous display name. A net may be
    /// named before wiring. UTF-8 names occupy 1..=65535 bytes.
    pub fn set_label(&mut self, net: NetId, label: String) -> Result<Option<String>, &'static str> {
        if label.is_empty() || label.len() > usize::from(u16::MAX) {
            return Err("net label must occupy 1..=65535 UTF-8 bytes");
        }
        Ok(self.labels.insert(net, label))
    }

    /// Remove the display name while preserving electrical connections.
    pub fn remove_label(&mut self, net: NetId) -> Option<String> {
        self.labels.remove(&net)
    }

    /// Borrow a net's display name.
    pub fn label(&self, net: NetId) -> Option<&str> {
        self.labels.get(&net).map(String::as_str)
    }

    /// Resolve a placed pin to its wired net and optional display name.
    pub fn probe_pin(&self, pin: PinId) -> Option<(NetId, Option<&str>)> {
        let net = self.pins.get(&pin)?.net?;
        Some((net, self.label(net)))
    }
}
