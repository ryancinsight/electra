use super::{StepError, Time};

pub(super) fn positive(value: f64) -> Result<(), StepError> {
    if value.is_finite() && value > 0. {
        Ok(())
    } else {
        Err(StepError::Component)
    }
}

pub(super) fn advance(state: f64, target: f64, tau: Time, dt: Time) -> Result<f64, StepError> {
    let dt = interval(dt)?;
    let tau = tau.into_base();
    if !state.is_finite() || !target.is_finite() {
        return Err(StepError::State);
    }
    if dt == 0. || state == target {
        return Ok(state);
    }
    // Normalize by the larger time to avoid overflowing tau + dt.
    let scale = dt.max(tau);
    let (a, b) = (tau / scale, dt / scale);
    let sum = a + b;
    let next = (a / sum).mul_add(state, (b / sum) * target);
    if next.is_finite() {
        Ok(next)
    } else {
        Err(StepError::State)
    }
}

pub(super) fn interval(dt: Time) -> Result<f64, StepError> {
    let dt = dt.into_base();
    if !dt.is_finite() || dt < 0. {
        return Err(StepError::Interval);
    }
    Ok(dt)
}
