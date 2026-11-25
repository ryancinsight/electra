//! Typed RMS voltage trace.
use electra_ac::{ElectricPotential, Frequency};
use iris::{IrisResult, view::SeriesView};

/// Borrow an RMS voltage spectrum. Callers supply RMS amplitudes in volts.
///
/// ```compile_fail,E0308
/// use electra_ac::{ElectricCurrent, Frequency};
/// use electra_iris_ac::rms_voltage_spectrum;
/// fn wrong(f: &[Frequency], current: &[ElectricCurrent]) {
///     let _ = rms_voltage_spectrum(f, current);
/// }
/// ```
pub fn rms_voltage_spectrum<'a>(
    frequencies: &'a [Frequency],
    voltages: &'a [ElectricPotential],
) -> IrisResult<SeriesView<'a, Frequency, ElectricPotential>> {
    SeriesView::new("RMS voltage (V)", frequencies, voltages)
}
