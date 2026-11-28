//! Outward-rounded arithmetic for the steady sheet residual.

const MAX_ROW_BRANCHES: u32 = 5;
const UNIT_ROUNDOFF: f64 = f64::EPSILON / 2.;
pub(super) const MIN_SUBNORMAL: f64 = f64::from_bits(1);

pub(super) fn row_roundoff(
    injection: f64,
    maximum_temperature: f64,
    conductance_sum: f64,
    branches: u32,
) -> Result<f64, String> {
    if branches > MAX_ROW_BRANCHES {
        return Err("thermal row exceeds its branch bound".into());
    }
    let operations = branches
        .checked_mul(3)
        .ok_or("thermal residual operation count overflow")?;
    let operations = f64::from(operations);
    let temperature_scale = upward_mul(upward_mul(2., maximum_temperature)?, conductance_sum)?;
    let scale = upward_add(injection, temperature_scale)?;
    // A row evaluates at most five g * (T_i - T_j) terms. The standard
    // gamma_n bound is below 2*n*u because n*u < 1/2 here. The second term
    // covers gradual-underflow error for every rounded operation. This
    // certifies the discrete system formed by the stored binary64 coefficients.
    let gamma = upward_mul(upward_mul(2., operations)?, UNIT_ROUNDOFF)?;
    let relative = upward_mul(gamma, scale)?;
    let underflow = upward_mul(upward_mul(2., operations)?, MIN_SUBNORMAL)?;
    upward_add(relative, underflow)
}

pub(super) fn upward_add(left: f64, right: f64) -> Result<f64, String> {
    next_up(left + right)
}

pub(super) fn upward_mul(left: f64, right: f64) -> Result<f64, String> {
    next_up(left * right)
}

pub(super) fn upward_div(numerator: f64, denominator: f64) -> Result<f64, String> {
    next_up(numerator / denominator)
}

fn next_up(value: f64) -> Result<f64, String> {
    if !value.is_finite() || value < 0. {
        return Err("thermal numerical certificate is not representable".into());
    }
    let bits = value
        .to_bits()
        .checked_add(1)
        .ok_or("thermal numerical certificate is not representable")?;
    let upper = f64::from_bits(bits);
    if upper.is_finite() {
        Ok(upper)
    } else {
        Err("thermal numerical certificate is not representable".into())
    }
}
