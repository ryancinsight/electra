use super::*;

/// Ideal synchronous buck with a series inductor and parallel capacitor/load.
/// The caller splits intervals at PWM edges. Reverse inductor current is allowed.
pub struct SynchronousBuck {
    l: f64,
    c: f64,
    r: f64,
}
impl SynchronousBuck {
    /// Create the output filter from strictly positive finite component values.
    pub fn new(l: Inductance, c: Capacitance, r: ElectricalImpedance) -> Result<Self, PowerError> {
        let (l, c, r) = (l.into_base(), c.into_base(), r.into_base());
        if [l, c, r].into_iter().all(positive) {
            Ok(Self { l, c, r })
        } else {
            Err(PowerError::Input)
        }
    }
    /// Implicit midpoint step with the high-side or low-side switch held closed.
    pub fn advance(
        &self,
        state: RlcState,
        bus: ElectricPotential,
        high: bool,
        dt: Time,
    ) -> Result<RlcState, PowerError> {
        let h = 0.5 * interval(dt)?;
        let (v, i, vs) = (
            state.voltage.into_base(),
            state.current.into_base(),
            bus.into_base(),
        );
        if ![v, i, vs].iter().all(|v| v.is_finite()) || vs < 0. {
            return Err(PowerError::Input);
        }
        let drive = if high { vs } else { 0. };
        let (a, b) = (h / self.l, h / self.c);
        if a == 0. || b == 0. {
            return Err(PowerError::TimestepExhausted);
        }
        let pivot = a.mul_add(b, 1. + b / self.r);
        if !pivot.is_finite() {
            return Err(PowerError::Range);
        }
        let mid_v = b.mul_add(a.mul_add(drive, i), v) / pivot;
        let mid_i = a.mul_add(drive - mid_v, i);
        let vn = mid_v + (mid_v - v);
        let next_i = mid_i + (mid_i - i);
        if !vn.is_finite() || !next_i.is_finite() {
            return Err(PowerError::Range);
        }
        Ok(RlcState {
            voltage: ElectricPotential::from_base(vn),
            current: ElectricCurrent::from_base(next_i),
        })
    }
}
