//! CSV outputs for voltage response and independent resistor noise.
use aequitas::systems::si::units::Degree;
use electra_ac::{LogSweep, RcLadder, noise::NoiseLadder};
use std::{error::Error, io::Write};

pub(super) fn response(
    out: &mut impl Write,
    grid: LogSweep,
    model: RcLadder<'_>,
) -> Result<(), Box<dyn Error>> {
    writeln!(
        out,
        "frequency_Hz,gain_re,gain_im,gain_magnitude,phase_deg,zout_re_ohm,zout_im_ohm"
    )?;
    for f in grid {
        let point = model.at(f?)?;
        let gain = point.gain.into_base();
        let z = point.output_impedance.into_base();
        let phase = point
            .phase()
            .ok_or("undefined output phase")?
            .in_unit::<Degree>();
        writeln!(
            out,
            "{},{},{},{},{},{},{}",
            point.frequency.into_base(),
            gain.re,
            gain.im,
            point.magnitude().into_base(),
            phase,
            z.re,
            z.im
        )?;
    }
    Ok(())
}

pub(super) fn noise(
    out: &mut impl Write,
    grid: LogSweep,
    model: NoiseLadder<'_>,
) -> Result<(), Box<dyn Error>> {
    writeln!(out, "frequency_Hz,output_voltage_psd_V2_per_Hz")?;
    for f in grid {
        let frequency = f?;
        writeln!(
            out,
            "{},{}",
            frequency.into_base(),
            model.at(frequency)?.into_base()
        )?;
    }
    Ok(())
}
