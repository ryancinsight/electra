//! Footprint role and legal rigid rotations.
/// Placement role — drives where a part *wants* to sit in the placement energy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// An active IC (HV7355 pulser, isolator) — pulled to the board core.
    ActiveIc,
    /// A board-edge interface (transducer / control / power connector) — pulled to the periphery.
    Connector,
    /// A decoupling/bypass capacitor — pinned next to its IC's power pin.
    Decoupling,
    /// A power-conversion part (buck, bulk cap).
    Power,
    /// A generic passive (series R, ferrite) — placed by wirelength only.
    Passive,
}

/// A 90° orientation step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rot {
    /// 0°.
    #[default]
    R0,
    /// 90° counter-clockwise.
    R90,
    /// 180°.
    R180,
    /// 270° counter-clockwise.
    R270,
}

impl Rot {
    /// Orientation in degrees (CCW), for emitting a KiCad footprint angle.
    #[must_use]
    pub fn degrees(self) -> f64 {
        match self {
            Rot::R0 => 0.0,
            Rot::R90 => 90.0,
            Rot::R180 => 180.0,
            Rot::R270 => 270.0,
        }
    }

    /// The next 90° step (cycles R0→R90→R180→R270→R0).
    #[must_use]
    pub fn next(self) -> Rot {
        match self {
            Rot::R0 => Rot::R90,
            Rot::R90 => Rot::R180,
            Rot::R180 => Rot::R270,
            Rot::R270 => Rot::R0,
        }
    }

    /// The 180° counterpart.
    #[must_use]
    pub fn opposite(self) -> Rot {
        match self {
            Rot::R0 => Rot::R180,
            Rot::R90 => Rot::R270,
            Rot::R180 => Rot::R0,
            Rot::R270 => Rot::R90,
        }
    }

    /// Next rotation allowed by a footprint policy, relative to the floorplanned orientation.
    #[must_use]
    pub fn next_allowed(self, initial: Rot, policy: RotationPolicy) -> Option<Rot> {
        match policy {
            RotationPolicy::Fixed => None,
            RotationPolicy::HalfTurn => Some(if self == initial {
                initial.opposite()
            } else {
                initial
            }),
            RotationPolicy::AnyRightAngle => Some(self.next()),
        }
    }

    /// Rotate an offset about the footprint centre.
    #[must_use]
    pub fn apply(self, p: crate::geom::Point) -> crate::geom::Point {
        match self {
            Rot::R0 => p,
            Rot::R90 => crate::geom::Point::new(-p.y, p.x),
            Rot::R180 => crate::geom::Point::new(-p.x, -p.y),
            Rot::R270 => crate::geom::Point::new(p.y, -p.x),
        }
    }

    /// Apply to a `(width, height)` courtyard size — swaps axes for the quarter turns.
    #[must_use]
    pub fn apply_size(
        self,
        size: (crate::geom::Nm, crate::geom::Nm),
    ) -> (crate::geom::Nm, crate::geom::Nm) {
        match self {
            Rot::R0 | Rot::R180 => size,
            Rot::R90 | Rot::R270 => (size.1, size.0),
        }
    }
}

/// Rotation freedom a footprint may use during placement optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationPolicy {
    /// Preserve the schematic/floorplan orientation exactly.
    Fixed,
    /// Permit only the original orientation and its 180° counterpart.
    HalfTurn,
    /// Permit all four right-angle orientations.
    AnyRightAngle,
}

impl RotationPolicy {
    /// Default policy for the placement role.
    #[must_use]
    pub fn for_role(role: Role) -> Self {
        match role {
            Role::ActiveIc | Role::Connector | Role::Power => RotationPolicy::Fixed,
            Role::Decoupling | Role::Passive => RotationPolicy::HalfTurn,
        }
    }
}
