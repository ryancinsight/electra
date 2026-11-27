//! Frequency-domain output voltage columns for schematic probes.
use electra_ac::{AcError, ElectricCurrent, ElectricPotential, Frequency, RcLadder};

/// Failure to produce a complete RC output trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcOutputError {
    /// Source RMS voltage was negative or nonfinite.
    Source,
    /// The solver rejected a frequency or encountered a numerical failure.
    Solver(AcError),
    /// The output voltage is outside the finite scalar range.
    VoltageRange,
    /// The terminal current is outside the finite scalar range.
    CurrentRange,
}

/// Solve the output RMS voltage at each borrowed frequency. Retains only
/// the voltage column, in input order, for use with the original frequency axis.
pub fn rc_output_rms(
    model: RcLadder<'_>,
    frequencies: &[Frequency],
    source_rms: ElectricPotential,
) -> Result<Vec<ElectricPotential>, RcOutputError> {
    collect_response(model, frequencies, source_rms, |point, source| {
        let voltage = source * point.magnitude().into_base();
        if voltage.is_finite() {
            Ok(ElectricPotential::from_base(voltage))
        } else {
            Err(RcOutputError::VoltageRange)
        }
    })
}

/// Terminal of the final resistive shunt. Positive current enters the component.
#[derive(Debug, Clone, Copy)]
pub enum ShuntTerminal {
    /// Terminal connected to the output net.
    Output,
    /// Return terminal connected to ground.
    Ground,
}

/// Compute the signed in-phase component of RMS terminal current, relative to
/// a real source phasor. The parallel capacitor is a separate branch.
pub fn rc_shunt_current_real(
    model: RcLadder<'_>,
    frequencies: &[Frequency],
    source_rms: ElectricPotential,
    terminal: ShuntTerminal,
) -> Result<Vec<ElectricCurrent>, RcOutputError> {
    let g = model.output_conductance().into_base();
    collect_response(model, frequencies, source_rms, |point, source| {
        let current = (source * point.gain.into_base().re) * g;
        if !current.is_finite() {
            return Err(RcOutputError::CurrentRange);
        }
        Ok(ElectricCurrent::from_base(match terminal {
            ShuntTerminal::Output => current,
            ShuntTerminal::Ground => -current,
        }))
    })
}

fn collect_response<T>(
    model: RcLadder<'_>,
    frequencies: &[Frequency],
    source_rms: ElectricPotential,
    project: impl Fn(electra_ac::AcPoint, f64) -> Result<T, RcOutputError>,
) -> Result<Vec<T>, RcOutputError> {
    let source = source_rms.into_base();
    if !source.is_finite() || source < 0. {
        return Err(RcOutputError::Source);
    }
    let mut output = Vec::with_capacity(frequencies.len());
    for &frequency in frequencies {
        output.push(project(
            model.at(frequency).map_err(RcOutputError::Solver)?,
            source,
        )?);
    }
    Ok(output)
}
