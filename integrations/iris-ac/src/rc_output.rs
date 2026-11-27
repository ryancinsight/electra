//! Frequency-domain output voltage columns for schematic probes.
use electra_ac::{AcError, ElectricPotential, Frequency, RcLadder};

/// Failure to produce a complete RC output trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcOutputError {
    /// Source RMS voltage was negative or nonfinite.
    Source,
    /// The solver rejected a frequency or encountered a numerical failure.
    Solver(AcError),
    /// The output voltage is outside the finite scalar range.
    VoltageRange,
}

/// Solve the output RMS voltage at each borrowed frequency. Retains only
/// the voltage column, in input order, for use with the original frequency axis.
pub fn rc_output_rms(
    model: RcLadder<'_>,
    frequencies: &[Frequency],
    source_rms: ElectricPotential,
) -> Result<Vec<ElectricPotential>, RcOutputError> {
    let source = source_rms.into_base();
    if !source.is_finite() || source < 0. {
        return Err(RcOutputError::Source);
    }
    let mut output = Vec::with_capacity(frequencies.len());
    for &frequency in frequencies {
        let point = model.at(frequency).map_err(RcOutputError::Solver)?;
        let voltage = source * point.magnitude().into_base();
        if !voltage.is_finite() {
            return Err(RcOutputError::VoltageRange);
        }
        output.push(ElectricPotential::from_base(voltage));
    }
    Ok(output)
}
