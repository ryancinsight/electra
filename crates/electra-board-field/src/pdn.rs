//! Board-geometry parasitics for capacitor-bank comparisons.
use electra_ac::{
    Capacitance, Complex64, ElectricalImpedance, Frequency, Inductance, Length,
    pdn::{BankError, SeriesCapacitor},
};

use crate::rail::Resistivity;

const VACUUM_PERMEABILITY: f64 = 4.0e-7 * std::f64::consts::PI;

/// Invalid geometry, component input, or unrepresentable impedance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PdnError {
    /// Every geometric length and material resistivity must be finite and positive.
    Geometry,
    /// A power/return pair requires a center pitch larger than its drill diameter.
    ViaClearance,
    /// Capacitance must be positive while component ESR and ESL must be nonnegative.
    Component,
    /// A feed impedance exceeds the numeric range.
    DynamicRange,
}

impl std::fmt::Display for PdnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Geometry => "via dimensions and resistivity must be finite and positive",
            Self::ViaClearance => "via-pair pitch must exceed drill diameter",
            Self::Component => "capacitor values must be finite and passive",
            Self::DynamicRange => "board impedance calculation exceeded the numeric range",
        })
    }
}

impl std::error::Error for PdnError {}

/// One plated power via and one plated return via through a homogeneous board.
#[derive(Clone, Copy, Debug)]
pub struct ViaPair {
    resistance: ElectricalImpedance,
    inductance: Inductance,
}

impl ViaPair {
    /// Derive barrel resistance and quasi-static round-conductor loop inductance.
    pub fn new(
        length: Length,
        drill_diameter: Length,
        plating_thickness: Length,
        center_pitch: Length,
        resistivity: Resistivity,
    ) -> Result<Self, PdnError> {
        let [length, drill, plating, pitch, rho] = [
            length.into_base(),
            drill_diameter.into_base(),
            plating_thickness.into_base(),
            center_pitch.into_base(),
            resistivity.into_base(),
        ];
        if ![length, drill, plating, pitch, rho]
            .into_iter()
            .all(|value| value.is_finite() && value > 0.0)
        {
            return Err(PdnError::Geometry);
        }
        if pitch <= drill {
            return Err(PdnError::ViaClearance);
        }
        let resistance = 2.0 * rho * length / (std::f64::consts::PI * drill * plating);
        let inductance =
            VACUUM_PERMEABILITY * length / std::f64::consts::PI * (pitch / drill).acosh();
        // Reject finite inputs whose derived parasitic was erased by underflow.
        if ![resistance, inductance]
            .into_iter()
            .all(|value| value.is_finite() && value > 0.0)
        {
            return Err(PdnError::DynamicRange);
        }
        Ok(Self {
            resistance: ElectricalImpedance::from_base(resistance),
            inductance: Inductance::from_base(inductance),
        })
    }

    /// Series impedance of the feed pair under the exp(+jωt) convention.
    pub fn impedance(
        self,
        frequency: Frequency,
    ) -> Result<ElectricalImpedance<Complex64>, PdnError> {
        let frequency = frequency.into_base();
        if !frequency.is_finite() || frequency <= 0.0 {
            return Err(PdnError::DynamicRange);
        }
        let reactance = std::f64::consts::TAU * frequency * self.inductance.into_base();
        if !reactance.is_finite() || reactance <= 0.0 {
            return Err(PdnError::DynamicRange);
        }
        Ok(ElectricalImpedance::from_base(Complex64::new(
            self.resistance.into_base(),
            reactance,
        )))
    }
}

/// Add one dedicated power/return via pair to a capacitor's ESR and ESL.
pub fn placed_capacitor(
    capacitance: Capacitance,
    component_esr: ElectricalImpedance,
    component_esl: Inductance,
    connection: ViaPair,
) -> Result<SeriesCapacitor, PdnError> {
    SeriesCapacitor::new(capacitance, component_esr, component_esl)
        .map_err(|_| PdnError::Component)?;
    let resistance = component_esr.into_base() + connection.resistance.into_base();
    let inductance = component_esl.into_base() + connection.inductance.into_base();
    if !resistance.is_finite()
        || !inductance.is_finite()
        || resistance == component_esr.into_base()
        || inductance == component_esl.into_base()
    {
        return Err(PdnError::DynamicRange);
    }
    SeriesCapacitor::new(
        capacitance,
        ElectricalImpedance::from_base(resistance),
        Inductance::from_base(inductance),
    )
    .map_err(|error| match error {
        BankError::DynamicRange | BankError::Frequency => PdnError::DynamicRange,
        BankError::Capacitance | BankError::Resistance | BankError::Inductance => {
            PdnError::Component
        }
    })
}
