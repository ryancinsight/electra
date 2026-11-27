//! Borrow Electra frequency-response and voltage-noise samples for Iris.
mod noise;
pub mod probe;
mod rc_output;
pub use rc_output::{
    RcOutputError, ShuntTerminal, rc_output_rms, rc_shunt_current_real, visit_rc_probe,
};
pub mod readback;
mod voltage;
use electra_ac::{Angle, Dimensionless, Frequency};
use iris::{IrisResult, view::SeriesView};
pub use noise::voltage_noise_spectrum;
pub use voltage::rms_voltage_spectrum;

/// Paired typed magnitude and phase traces over the same frequency slice.
pub struct BodeView<'a> {
    magnitude: SeriesView<'a, Frequency, Dimensionless>,
    phase: SeriesView<'a, Frequency, Angle>,
}
impl<'a> BodeView<'a> {
    /// Borrow all three axes. Iris validates both ordinate lengths against frequency.
    ///
    /// ```compile_fail
    /// use electra_ac::{Angle, ElectricPotential, Frequency};
    /// use electra_iris_ac::BodeView;
    /// fn wrong(f: &[Frequency], v: &[ElectricPotential], phase: &[Angle]) {
    ///     let _ = BodeView::new(f, v, phase);
    /// }
    /// ```
    pub fn new(
        frequencies: &'a [Frequency],
        gains: &'a [Dimensionless],
        phases: &'a [Angle],
    ) -> IrisResult<Self> {
        Ok(Self {
            magnitude: SeriesView::new("Voltage gain", frequencies, gains)?,
            phase: SeriesView::new("Phase", frequencies, phases)?,
        })
    }
    /// Borrow the linear voltage ratio series for rendering.
    pub fn magnitude(&self) -> &SeriesView<'a, Frequency, Dimensionless> {
        &self.magnitude
    }
    /// Borrow the principal phase series for rendering.
    pub fn phase(&self) -> &SeriesView<'a, Frequency, Angle> {
        &self.phase
    }
    /// Read the original solver coordinate and both ordinates at a selected sample.
    pub fn sample(&self, index: usize) -> Option<(Frequency, Dimensionless, Angle)> {
        let (frequency, gain) = self.magnitude.sample_at(index)?;
        let (_, phase) = self.phase.sample_at(index)?;
        Some((*frequency, *gain, *phase))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use electra_ac::{Capacitance, ElectricConductance, ElectricalImpedance, RcLadder, Stage};
    #[test]
    fn cursor_reads_the_original_solver_coordinate() {
        let cells = [Stage::new(
            ElectricalImpedance::from_base(1000.),
            Capacitance::from_base(1. / (std::f64::consts::TAU * 1e6)),
            ElectricConductance::default(),
        )
        .unwrap()];
        let f = [100., 1000., 10000.].map(Frequency::from_base);
        let ladder = RcLadder::new(&cells).unwrap();
        let points = f.map(|f| ladder.at(f).unwrap());
        let gain = points.map(|p| p.magnitude());
        let phase = points.map(|p| p.phase().unwrap());
        let view = BodeView::new(&f, &gain, &phase).unwrap();
        assert!(core::ptr::eq(
            view.magnitude().coordinates().as_ptr(),
            f.as_ptr()
        ));
        assert!(core::ptr::eq(
            view.phase().samples().as_ptr(),
            phase.as_ptr()
        ));
        let (cursor, mag, angle) = view.sample(1).unwrap();
        assert_eq!(cursor, points[1].frequency);
        assert_eq!(cursor.into_base(), 1000.);
        assert!((mag.into_base() - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-14);
        assert!((angle.into_base() + std::f64::consts::FRAC_PI_4).abs() < 1e-14);
        assert!(view.sample(3).is_none());
        assert!(BodeView::new(&f, &gain[..2], &phase).is_err());
        assert!(BodeView::new(&f, &gain, &phase[..2]).is_err());
    }
}
