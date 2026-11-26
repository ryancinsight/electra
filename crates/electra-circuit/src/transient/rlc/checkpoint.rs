//! Versioned fixed-step restart format with no native layout dependency.
use super::{
    Capacitance, ElectricCurrent, ElectricPotential, ElectricalImpedance, Inductance, RlcState,
    SeriesRlc, Time,
};
use std::io::{Error, ErrorKind};

const HEADER: &[u8; 8] = b"ERLC0001";
const SIZE: usize = 64;

/// Restored series branch and fixed-step cursor. Supply the same external drive schedule
/// starting at `next_step` to continue the trajectory.
pub struct RlcCheckpoint {
    /// Original component parameters.
    pub model: SeriesRlc,
    /// State at the start of the next interval.
    pub state: RlcState,
    /// Duration of each interval.
    pub interval: Time,
    /// Zero-based index of the next interval.
    pub next_step: u64,
}

fn invalid() -> Error {
    Error::new(ErrorKind::InvalidData, "invalid fixed-step RLC checkpoint")
}

fn validate(state: RlcState, interval: Time) -> Result<(), Error> {
    if state.voltage.into_base().is_finite()
        && state.current.into_base().is_finite()
        && interval.into_base().is_finite()
        && interval.into_base() > 0.
    {
        Ok(())
    } else {
        Err(invalid())
    }
}

impl SeriesRlc {
    /// Encode a fixed-step checkpoint as a header, u64 cursor and six little-endian
    /// f64 values in SI units (R, L, C, interval, voltage, current).
    pub fn checkpoint(
        &self,
        state: RlcState,
        interval: Time,
        next_step: u64,
    ) -> Result<[u8; SIZE], Error> {
        validate(state, interval)?;
        let mut bytes = [0; SIZE];
        bytes[..8].copy_from_slice(HEADER);
        bytes[8..16].copy_from_slice(&next_step.to_le_bytes());
        for (slot, value) in bytes[16..].chunks_exact_mut(8).zip([
            self.resistance.into_base(),
            self.inductance.into_base(),
            self.capacitance.into_base(),
            interval.into_base(),
            state.voltage.into_base(),
            state.current.into_base(),
        ]) {
            slot.copy_from_slice(&value.to_le_bytes());
        }
        Ok(bytes)
    }
}

impl RlcCheckpoint {
    /// Decode a complete record, checking version, length and physical parameter ranges.
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != SIZE || &bytes[..8] != HEADER {
            return Err(invalid());
        }
        let mut fields = bytes[8..]
            .chunks_exact(8)
            .map(|field| <[u8; 8]>::try_from(field).unwrap());
        let next_step = u64::from_le_bytes(fields.next().unwrap());
        let mut number = || f64::from_le_bytes(fields.next().unwrap());
        let model = SeriesRlc::new(
            ElectricalImpedance::from_base(number()),
            Inductance::from_base(number()),
            Capacitance::from_base(number()),
        )
        .map_err(|_| invalid())?;
        let interval = Time::from_base(number());
        let state = RlcState {
            voltage: ElectricPotential::from_base(number()),
            current: ElectricCurrent::from_base(number()),
        };
        validate(state, interval)?;
        Ok(Self {
            model,
            state,
            interval,
            next_step,
        })
    }
}
