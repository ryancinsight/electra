use core::fmt;

/// Model, sweep or numerical failure.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AcError {
    /// A series resistance must be positive and finite.
    Resistance,
    /// A shunt capacitance must be nonnegative and finite.
    Capacitance,
    /// A shunt conductance must be nonnegative and finite.
    Conductance,
    /// The ladder must contain at least one stage.
    Empty,
    /// Point frequencies must be finite and nonnegative.
    Frequency,
    /// A logarithmic sweep needs positive endpoints and at least two distinct samples,
    /// or one sample with equal endpoints.
    Grid,
    /// The stage calculation exceeded the representable range of the scalar.
    DynamicRange {
        /// Zero-based index in input-to-output stage order.
        stage: usize,
    },
}
impl fmt::Display for AcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Resistance => f.write_str("series resistance must be finite and positive"),
            Self::Capacitance => f.write_str("shunt capacitance must be finite and nonnegative"),
            Self::Conductance => f.write_str("shunt conductance must be finite and nonnegative"),
            Self::Empty => f.write_str("at least one RC stage is required"),
            Self::Frequency => f.write_str("frequency must be finite and nonnegative"),
            Self::Grid => f.write_str("log sweep needs positive endpoints, a valid sample count and distinguishable frequencies"),
            Self::DynamicRange { stage } => write!(f, "AC arithmetic exceeds the numeric range at stage {stage}"),
        }
    }
}
impl std::error::Error for AcError {}
