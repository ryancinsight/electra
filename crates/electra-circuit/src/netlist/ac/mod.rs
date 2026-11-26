//! Flat passive AC decks with prescribed phasor sources.
mod system;
pub use system::AcSystem;
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

    /// Visit oriented branch currents in deck order. Each callback receives
    /// the source line, endpoint indices, and complex current in amperes.
    pub fn branch_currents(
        &self,
        frequency: Frequency,
        volts: &[ElectricPotential<Complex64>],
        mut visit: impl FnMut(usize, [usize; 2], ElectricCurrent<Complex64>),
    ) -> Result<(), ParseError> {
        if volts.len() != self.names.len() {
            return Err(ParseError::new(
                1,
                "voltage vector length differs from node count",
            ));
        }
        for &(line, branch) in &self.branches {
            let value = branch
                .current(frequency, volts)
                .map_err(|e| ParseError::new(line, e.to_string()))?;
            visit(line, branch.endpoints(), value);
        }
        Ok(())
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
