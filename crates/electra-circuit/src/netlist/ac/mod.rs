//! Flat passive AC decks with prescribed phasor sources.
mod parse;
mod source;
use super::ParseError;
use electra_ac::{
    Complex64, ElectricConductance, ElectricCurrent, ElectricPotential, Frequency,
    nodal::PassiveBranch,
};
pub use parse::parse_ac;

/// Node order, prescribed voltages, and validated passive branches reusable across frequencies.
#[derive(Debug)]
pub struct AcDeck {
    names: Vec<String>,
    boundary: Vec<Option<ElectricPotential<Complex64>>>,
    branches: Vec<(usize, PassiveBranch)>,
}
impl AcDeck {
    /// Node labels in stamp index order. Node zero is ground.
    pub fn nodes(&self) -> &[String] {
        &self.names
    }

    /// Known voltage at each prescribed node. Free nodes contain None.
    pub fn boundary(&self) -> &[Option<ElectricPotential<Complex64>>] {
        &self.boundary
    }

    /// Emit additive SI coefficients for one frequency. Discard contributions
    /// from this invocation if a later branch reports a line-numbered error.
    pub fn stamp(
        &self,
        frequency: Frequency,
        mut matrix: impl FnMut(usize, usize, ElectricConductance<Complex64>),
        mut rhs: impl FnMut(usize, ElectricCurrent<Complex64>),
    ) -> Result<(), ParseError> {
        for &(line, branch) in &self.branches {
            branch
                .stamp(frequency, &self.boundary, &mut matrix, &mut rhs)
                .map_err(|e| ParseError::new(line, e.to_string()))?;
        }
        Ok(())
    }
}
