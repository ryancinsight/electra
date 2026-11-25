//! Normalized scalar kernels used by both reflected impedance and voltage gain.
use crate::{
    Complex64, Dimensionless, ElectricalImpedance, Frequency, Inductance, magnetics::MagneticError,
};
use aequitas::Quantity;

pub(super) fn open_gain(
    primary: Inductance,
    secondary: Inductance,
    coupling: Dimensionless,
) -> Result<Dimensionless, MagneticError> {
    let lp = primary.into_base().sqrt();
    let ls = secondary.into_base().sqrt();
    let k = coupling.into_base();
    let ratio = ls / lp;
    let gain = if ratio.is_finite() {
        k * ratio
    } else {
        (k * ls) / lp
    };
    if !gain.is_finite() || gain == 0. {
        return Err(MagneticError::DynamicRange);
    }
    Ok(Dimensionless::from_base(gain))
}

pub(super) fn reactance(
    f: Frequency,
    inductance: Inductance,
) -> Result<ElectricalImpedance, MagneticError> {
    let product: ElectricalImpedance = f * inductance;
    let tau = std::f64::consts::TAU;
    let x = if product.into_base() < f64::MIN_POSITIVE {
        // Round only after the angular factor when fL is subnormal.
        if f.into_base() >= inductance.into_base() {
            (f * tau) * inductance
        } else {
            f * (inductance * tau)
        }
    } else {
        product * tau
    };
    if x.as_base().is_finite() && x.into_base() > 0. {
        Ok(x)
    } else {
        Err(MagneticError::DynamicRange)
    }
}

/// Multiply by a bounded ratio without losing a representable product when the ratio underflows.
#[inline]
fn weighted(scale: f64, small: f64, large: f64, ratio: f64) -> f64 {
    if ratio >= f64::MIN_POSITIVE || small == 0. || scale.abs() <= 1. {
        scale * ratio
    } else if small <= 1. {
        (scale * small) / large
    } else {
        (scale / large) * small
    }
}

/// Return scale × R/(R+jX). The dimension specializes to voltage ratio or ohms.
#[inline]
pub(super) fn loaded<D>(
    scale: Quantity<f64, D>,
    resistance: ElectricalImpedance,
    reactance: ElectricalImpedance,
) -> Quantity<Complex64, D> {
    let s = scale.into_base();
    let r = resistance.into_base();
    let x = reactance.into_base();
    let z = if x <= r {
        let t = x / r;
        let re = s / t.mul_add(t, 1.);
        Complex64::new(re, -weighted(re, x, r, t))
    } else {
        let t = r / x;
        let lag = weighted(s, r, x, t) / t.mul_add(t, 1.);
        Complex64::new(weighted(lag, r, x, t), -lag)
    };
    Quantity::from_base(z)
}
