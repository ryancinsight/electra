/// Passive-device preparation or nodal stamp error.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum StampError {
    /// Invalid resistance.
    Resistance,
    /// Invalid capacitance.
    Capacitance,
    /// Invalid inductance.
    Inductance,
    /// The node index exceeds the boundary slice.
    Node(usize),
    /// A prescribed phasor is nonfinite.
    Boundary(usize),
    /// Frequency must be finite and nonnegative.
    Frequency,
    /// A DC inductor needs an ideal short constraint.
    DcInductor,
    /// A coefficient or prescribed-voltage contribution exceeds numeric range.
    Range,
}

impl core::fmt::Display for StampError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Node(i) => write!(f, "unknown AC node {i}"),
            Self::Boundary(i) => write!(f, "nonfinite prescribed voltage at node {i}"),
            Self::Resistance => f.write_str("resistance must be finite and positive"),
            Self::Capacitance => f.write_str("capacitance must be finite and nonnegative"),
            Self::Inductance => f.write_str("inductance must be finite and positive"),
            Self::Frequency => f.write_str("frequency must be finite and nonnegative"),
            Self::DcInductor => f.write_str("DC inductor requires an ideal short constraint"),
            Self::Range => f.write_str("AC stamp exceeds numeric range"),
        }
    }
}
impl std::error::Error for StampError {}
