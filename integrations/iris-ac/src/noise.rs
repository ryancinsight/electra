//! Typed one-sided voltage-noise trace.
use electra_ac::{Frequency, noise::VoltageNoisePsd};
use iris::{IrisResult, view::SeriesView};

/// Borrow the solver's frequencies in Hz and one-sided voltage PSD in V²/Hz.
/// Coordinates and samples are available by index for cursor readback.
///
/// ```compile_fail
/// use electra_ac::{ElectricPotential, Frequency};
/// use electra_iris_ac::voltage_noise_spectrum;
/// fn wrong(f: &[Frequency], voltage: &[ElectricPotential]) {
///     let _ = voltage_noise_spectrum(f, voltage);
/// }
/// ```
pub fn voltage_noise_spectrum<'a>(
    frequencies: &'a [Frequency],
    densities: &'a [VoltageNoisePsd],
) -> IrisResult<SeriesView<'a, Frequency, VoltageNoisePsd>> {
    SeriesView::new("Output voltage PSD (V²/Hz)", frequencies, densities)
}

#[cfg(test)]
mod tests {
    use super::*;
    use electra_ac::{
        Capacitance, ElectricConductance, ElectricalImpedance, Stage, ThermodynamicTemperature,
        noise::{NoiseLadder, ThermalStage},
    };

    #[test]
    fn cursor_reads_the_pole_density_from_borrowed_solver_samples() {
        let cell = Stage::new(
            ElectricalImpedance::from_base(1000.),
            Capacitance::from_base(1. / (std::f64::consts::TAU * 1e6)),
            ElectricConductance::default(),
        )
        .unwrap();
        let cells = [ThermalStage::new(
            cell,
            ThermodynamicTemperature::from_base(300.),
            ThermodynamicTemperature::default(),
        )
        .unwrap()];
        let model = NoiseLadder::new(&cells).unwrap();
        let f = [100., 1000., 10000.].map(Frequency::from_base);
        let psd = f.map(|f| model.at(f).unwrap());
        let view = voltage_noise_spectrum(&f, &psd).unwrap();
        assert!(std::ptr::eq(view.coordinates().as_ptr(), f.as_ptr()));
        assert!(std::ptr::eq(view.samples().as_ptr(), psd.as_ptr()));
        assert_eq!(view.coordinates()[1].into_base(), 1000.);
        assert!((view.samples()[1].into_base() / 8.283894e-18 - 1.).abs() < 1e-14);
        assert!(voltage_noise_spectrum(&f, &psd[..2]).is_err());
    }
}
