//! Net, pad, layer, track, via and copper zone records.
use crate::geom::{Nm, Point};
/// A copper-layer index. Layer `0` is the top (F.Cu); `nlayers - 1` is the bottom (B.Cu).
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LayerId(pub u16);

/// A net identifier.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NetId(pub u32);

/// The electrical class of a net — a closed set, so it is dispatched as an exhaustive enum rather
/// than a trait object. The class drives both design-rule selection and physics cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NetClassKind {
    /// High-voltage net (VPP/VDDH/TX outputs) — wide track, large clearance, creepage-policed.
    Hv,
    /// Low-voltage logic/control signal.
    Signal,
    /// A power rail (5 V, 3.3 V) routed as wide copper or a plane.
    Power,
    /// Ground / return.
    Ground,
}

/// Analog/digital floorplan domain inferred from schematic net naming.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDomain {
    /// Analog section / analog return.
    Analog,
    /// Digital section / digital return.
    Digital,
}

impl NetClassKind {
    /// Whether this class is a low-voltage domain (the side HV must keep creepage from).
    #[inline]
    #[must_use]
    pub fn is_low_voltage(self) -> bool {
        matches!(
            self,
            NetClassKind::Signal | NetClassKind::Power | NetClassKind::Ground
        )
    }
}

/// Classify schematic net names into analog/digital floorplan domains.
#[must_use]
pub fn split_domain_from_name(name: &str) -> Option<SplitDomain> {
    let mut digital = name
        .get(..3)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("CLK"));
    for token in name.split(|c: char| !c.is_ascii_alphanumeric()) {
        if ["AGND", "AGROUND", "ANALOG", "ANA"]
            .iter()
            .any(|s| token.eq_ignore_ascii_case(s))
        {
            return Some(SplitDomain::Analog);
        }
        digital |= [
            "DGND", "DGROUND", "DIGITAL", "DIG", "FPGA", "SPI", "I2C", "UART",
        ]
        .iter()
        .any(|s| token.eq_ignore_ascii_case(s));
    }
    digital.then_some(SplitDomain::Digital)
}

/// A named net with an electrical class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Net {
    /// Stable identifier.
    pub id: NetId,
    /// Net name as it appears in the schematic.
    pub name: String,
    /// Electrical class.
    pub class: NetClassKind,
}

/// A component pad — a routing terminal. SMD pads list a single accessible layer; through-hole
/// pads list every layer they connect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pad {
    /// Pad centre on the board.
    pub pos: Point,
    /// Copper layers on which this pad is accessible.
    pub layers: Vec<LayerId>,
    /// Net this pad belongs to, if any (unconnected pads are `None`).
    pub net: Option<NetId>,
}

/// A routed copper track segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Track {
    /// Segment start.
    pub start: Point,
    /// Segment end.
    pub end: Point,
    /// Track width.
    pub width: Nm,
    /// Layer the segment is on.
    pub layer: LayerId,
    /// Net the segment carries.
    pub net: NetId,
}

/// Construction class of a via — the HDI pathway it represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViaKind {
    /// Mechanically drilled, spanning the full stack outer-to-outer.
    Through,
    /// Mechanically drilled, one outer layer to an inner layer (or non-adjacent inner span touching
    /// an outer).
    Blind,
    /// Mechanically drilled, inner-to-inner only (does not reach an outer layer).
    Buried,
    /// Laser-drilled micro-via between two *adjacent* layers touching an outer surface — the HDI
    /// build-up via; small diameter, low aspect ratio.
    Micro,
}

/// A routed via connecting two layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Via {
    /// Via centre.
    pub pos: Point,
    /// Drill (hole) diameter.
    pub drill: Nm,
    /// Outer pad diameter.
    pub diameter: Nm,
    /// Net the via carries.
    pub net: NetId,
    /// Span: lower layer.
    pub from: LayerId,
    /// Span: upper layer.
    pub to: LayerId,
    /// Construction class (through / blind / buried / micro) — set from the layer span by
    /// [`Via::classify`].
    pub kind: ViaKind,
    /// **VIPPO** — via-in-pad, plated-over (filled + capped). True for an escape via sitting inside a
    /// solderable pad (BGA ball / fine-pitch lead), so the pad stays flat and solderable.
    pub filled: bool,
}

impl Via {
    /// Classify a via from its layer span on an `nlayers` stack: outer-to-outer ⇒ `Through`; a single
    /// adjacent hop touching an outer surface ⇒ `Micro` (laser build-up via); any other span that
    /// reaches an outer layer ⇒ `Blind`; an inner-only span ⇒ `Buried`.
    #[must_use]
    pub fn classify(from: LayerId, to: LayerId, nlayers: usize) -> ViaKind {
        let (lo, hi) = (from.0.min(to.0), from.0.max(to.0));
        let last = (nlayers - 1) as u16;
        let touches_outer = lo == 0 || hi == last;
        if lo == 0 && hi == last {
            ViaKind::Through
        } else if hi - lo == 1 && touches_outer {
            ViaKind::Micro
        } else if touches_outer {
            ViaKind::Blind
        } else {
            ViaKind::Buried
        }
    }
}

/// How a copper [`Zone`]'s fill connects to same-net pads — the manufacturing trade-off between
/// solderability (thermal relief spokes) and a solid low-impedance connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneFill {
    /// Thermal-relief spokes: a small gap with bridging spokes around pads, so reflow heat is not
    /// sunk into the plane. The right choice for a ground/power **pour**.
    ThermalRelief,
    /// Solid: copper fills right up to the pad. The right choice for a **teardrop** fillet, whose
    /// whole purpose is a continuous mechanical/electrical transition into the pad.
    Solid,
}

/// A filled copper region on one layer, assigned to a net. KiCad's zone filler carves design-rule
/// clearance around every foreign-net feature automatically, so a zone is the clearance-safe way to
/// add copper (ground pour for EMI return / shielding / copper balance, or teardrop reinforcement).
#[derive(Debug, Clone)]
pub struct Zone {
    /// Net the fill belongs to.
    pub net: NetId,
    /// Copper layer the fill sits on.
    pub layer: LayerId,
    /// Closed outline polygon (board-space points; the filler intersects it with copper rules).
    pub polygon: Vec<Point>,
    /// Pad-connection style.
    pub fill: ZoneFill,
}
