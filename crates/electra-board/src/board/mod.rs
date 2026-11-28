//! Copper storage parameterized by the caller’s routing grid.
mod copper;
pub use copper::*;
/// A board: its routing grid, electrical nets, component pads, and the routed copper produced by
/// the engine.
#[derive(Debug, Clone)]
pub struct Board<S> {
    /// The routing grid that discretises this board.
    pub spec: S,
    /// All nets, indexed by `NetId.0`.
    pub nets: Vec<Net>,
    /// All component pads.
    pub pads: Vec<Pad>,
    /// Routed track segments (filled by the router).
    pub tracks: Vec<Track>,
    /// Routed vias (filled by the router).
    pub vias: Vec<Via>,
    /// Copper fill regions (ground pour, teardrops) — emitted as KiCad zones.
    pub zones: Vec<Zone>,
}

impl<S> Board<S> {
    /// Create an empty board over the given routing grid.
    #[must_use]
    pub fn new(spec: S) -> Self {
        Board {
            spec,
            nets: Vec::new(),
            pads: Vec::new(),
            tracks: Vec::new(),
            vias: Vec::new(),
            zones: Vec::new(),
        }
    }

    /// Find a net by exact name (e.g. the ground net for a copper pour).
    #[must_use]
    pub fn net_by_name(&self, name: &str) -> Option<NetId> {
        self.nets.iter().find(|n| n.name == name).map(|n| n.id)
    }

    /// Add a net, returning its id.
    pub fn add_net(&mut self, name: impl Into<String>, class: NetClassKind) -> NetId {
        let id = NetId(self.nets.len() as u32);
        self.nets.push(Net {
            id,
            name: name.into(),
            class,
        });
        id
    }

    /// Add a pad.
    pub fn add_pad(&mut self, pad: Pad) {
        self.pads.push(pad);
    }

    /// Electrical class of a net.
    #[inline]
    #[must_use]
    pub fn class_of(&self, net: NetId) -> NetClassKind {
        self.nets[net.0 as usize].class
    }

    /// Pads belonging to a net.
    #[inline]
    pub fn pads_of(&self, net: NetId) -> impl Iterator<Item = &Pad> {
        self.pads.iter().filter(move |p| p.net == Some(net))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn net_names_classify_split_domains() {
        assert_eq!(
            split_domain_from_name("ANALOG_SIG"),
            Some(SplitDomain::Analog),
            "explicit ANALOG token maps to the analog placement domain"
        );
        assert_eq!(
            split_domain_from_name("BUS_SPI_MOSI"),
            Some(SplitDomain::Digital),
            "SPI control nets map to the digital placement domain"
        );
        assert_eq!(
            split_domain_from_name("CLK_MAIN"),
            Some(SplitDomain::Digital),
            "clock nets map to the digital placement domain"
        );
        assert_eq!(
            split_domain_from_name("TX_0"),
            None,
            "unmarked high-speed output nets are not forced into an analog/digital split domain"
        );
    }
}
