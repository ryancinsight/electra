//! A componentwise backward-error check on the original complex equations.
use electra_ac::{Complex64, ElectricCurrent, ElectricPotential, PhasorSum};
use electra_circuit::AcSystem;
use std::error::Error;
pub(super) fn check(
    system: &AcSystem,
    v: &[ElectricPotential<Complex64>],
) -> Result<(), Box<dyn Error>> {
    let mut sums = vec![PhasorSum::default(); v.len()];
    let mut scales = vec![0.; v.len()];
    for (i, b) in system.currents().enumerate() {
        let b = b.into_base();
        sums[i].add(-b);
        scales[i] = b.re.hypot(b.im);
    }
    for (i, j, y) in system.coefficients() {
        let term = (y * v[j]).into_base();
        sums[i].add(term);
        scales[i] += term.re.hypot(term.im);
    }

    for (sum, scale) in sums.into_iter().zip(scales) {
        let r = sum.total();
        let err = r.re.hypot(r.im);
        if !err.is_finite() || !scale.is_finite() || err > 1e-10 * scale {
            return Err("AC solution failed unscaled KCL check".into());
        }
    }
    Ok(())
}

pub(super) fn physical(
    deck: &electra_circuit::AcDeck,
    frequency: electra_ac::Frequency,
    v: &[ElectricPotential<Complex64>],
) -> Result<ElectricCurrent, Box<dyn Error>> {
    let mut rows = vec![(PhasorSum::default(), 0.); v.len()];
    deck.branch_currents(frequency, v, |_, [from, to], amps| {
        let z = amps.into_base();
        let magnitude = z.re.hypot(z.im);
        for (i, signed) in [(from, z), (to, -z)] {
            if deck.boundary()[i].is_none() {
                rows[i].0.add(signed);
                rows[i].1 += magnitude;
            }
        }
    })?;
    let mut worst = 0f64;
    for (i, (sum, scale)) in rows.into_iter().enumerate() {
        let z = sum.total();
        let imbalance = z.re.hypot(z.im);
        if !imbalance.is_finite() || !scale.is_finite() || imbalance > 1e-12 + 1e-10 * scale {
            return Err(format!(
                "physical AC KCL failed at {} ({imbalance} A)",
                deck.nodes()[i]
            )
            .into());
        }
        worst = worst.max(imbalance);
    }
    Ok(ElectricCurrent::from_base(worst))
}
