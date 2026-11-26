//! Bounded complex AC operating points using the shared pivoted LU provider.
mod block;
mod components;
mod residual;
use super::PivotedLu;
use electra_ac::{Complex64, ElectricCurrent, ElectricPotential, Frequency};
use electra_circuit::{AcDeck, direct::DirectFactor};
use std::error::Error;

/// Original-node phasor voltages and worst unscaled nodal current imbalance.
pub struct AcPoint {
    /// Complex voltages in deck node order, including prescribed voltages.
    pub volts: Vec<ElectricPotential<Complex64>>,
    /// Largest absolute free-node KCL residual in amperes.
    pub residual: ElectricCurrent,
}

/// Solve a parsed passive deck at one frequency. The dense factor is bounded
/// to 128 free complex voltages per connected component. An unanchored component is rejected even
/// with zero excitation. Each free equation is checked in original SI units.
pub fn solve(deck: &AcDeck, frequency: Frequency) -> Result<AcPoint, Box<dyn Error>> {
    let system = deck.assemble(frequency)?;
    let mut volts: Vec<_> = deck
        .boundary()
        .iter()
        .map(|v| v.unwrap_or_default())
        .collect();
    components::visit(deck, &system, PivotedLu::MAX_ORDER / 2, |free, slots| {
        let block::Block { matrix, rhs } = block::Block::new(&system, free, slots)?;
        let order = 2 * free.len();
        let factor = PivotedLu::from_values(order, matrix)?;
        let mut x = vec![0.; order];
        factor.solve(&rhs, &mut x)?;
        for (pair, &node) in x.chunks_exact(2).zip(free) {
            if !pair[0].is_finite() || !pair[1].is_finite() {
                return Err("nonfinite AC voltage".into());
            }
            volts[node] = ElectricPotential::from_base(Complex64::new(pair[0], pair[1]));
        }
        Ok(())
    })?;
    residual::check(&system, &volts)?;
    let residual = residual::physical(deck, frequency, &volts)?;
    Ok(AcPoint { volts, residual })
}
