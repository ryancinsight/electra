//! Frequency-domain output voltage columns for schematic probes.
use electra_ac::{AcError, AcPoint, ElectricCurrent, ElectricPotential, Frequency, RcLadder};

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
    let mut output = Vec::new();
    visit_response(model, frequencies, source_rms, |point, source| {
        if output.is_empty() {
            output.reserve_exact(frequencies.len());
        }
        output.push(voltage(point, source)?);
        Ok(())
    })?;
    Ok(output)
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
    let mut output = Vec::new();
    visit_response(model, frequencies, source_rms, |point, source| {
        if output.is_empty() {
            output.reserve_exact(frequencies.len());
        }
        output.push(current(point, source, g, terminal)?);
        Ok(())
    })?;
    Ok(output)
}

/// Visit each solved frequency once, emitting its voltage and in-phase shunt
/// current together, in input order. Uses no trace buffers. On solver or
/// projection failure, the caller retains samples already delivered.
pub fn visit_rc_probe(
    model: RcLadder<'_>,
    frequencies: &[Frequency],
    source_rms: ElectricPotential,
    terminal: ShuntTerminal,
    mut sample: impl FnMut(Frequency, ElectricPotential, ElectricCurrent),
) -> Result<(), RcOutputError> {
    let g = model.output_conductance().into_base();
    visit_response(model, frequencies, source_rms, |point, source| {
        sample(
            point.frequency,
            voltage(point, source)?,
            current(point, source, g, terminal)?,
        );
        Ok(())
    })
}

fn voltage(point: AcPoint, source: f64) -> Result<ElectricPotential, RcOutputError> {
    let voltage = source * point.magnitude().into_base();
    if voltage.is_finite() {
        Ok(ElectricPotential::from_base(voltage))
    } else {
        Err(RcOutputError::VoltageRange)
    }
}

fn current(
    point: AcPoint,
    source: f64,
    g: f64,
    terminal: ShuntTerminal,
) -> Result<ElectricCurrent, RcOutputError> {
    let current = (source * point.gain.into_base().re) * g;
    if !current.is_finite() {
        return Err(RcOutputError::CurrentRange);
    }
    Ok(ElectricCurrent::from_base(match terminal {
        ShuntTerminal::Output => current,
        ShuntTerminal::Ground => -current,
    }))
}

fn visit_response(
    model: RcLadder<'_>,
    frequencies: &[Frequency],
    source_rms: ElectricPotential,
    mut visit: impl FnMut(AcPoint, f64) -> Result<(), RcOutputError>,
) -> Result<(), RcOutputError> {
    let source = source_rms.into_base();
    if !source.is_finite() || source < 0. {
        return Err(RcOutputError::Source);
    }
    for &frequency in frequencies {
        visit(model.at(frequency).map_err(RcOutputError::Solver)?, source)?;
    }
    Ok(())
}
