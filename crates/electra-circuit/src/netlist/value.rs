//! Physical values at the deck boundary. Formal dimensions survive through binding.
use super::number::magnitude;
use crate::{ElectricCurrent, ElectricPotential, ElectricalImpedance};
use aequitas::systems::si::quantities::Time;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Unit {
    Ohm,
    Volt,
    Ampere,
    Second,
}

impl Unit {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Ohm => "Ohm",
            Self::Volt => "V",
            Self::Ampere => "A",
            Self::Second => "s",
        }
    }
    pub fn device(name: &str) -> Result<Self, String> {
        match name.as_bytes().first().map(u8::to_ascii_uppercase) {
            Some(b'R') => Ok(Self::Ohm),
            Some(b'V') => Ok(Self::Volt),
            Some(b'I') => Ok(Self::Ampere),
            _ => Err(format!("unsupported element {name}")),
        }
    }
    pub fn value(self, token: &str) -> Result<Value, String> {
        let value = magnitude(token)
            .ok_or_else(|| format!("invalid finite {} magnitude '{token}'", self.symbol()))?;
        Ok(match self {
            Self::Ohm => Value::Resistance(ElectricalImpedance::from_base(value)),
            Self::Volt => Value::Voltage(ElectricPotential::from_base(value)),
            Self::Ampere => Value::Current(ElectricCurrent::from_base(value)),
            Self::Second => Value::Time(Time::from_base(value)),
        })
    }
}

#[derive(Clone, Copy)]
pub(super) enum Value {
    Resistance(ElectricalImpedance),
    Voltage(ElectricPotential),
    Current(ElectricCurrent),
    Time(Time),
}
impl Value {
    pub fn unit(self) -> Unit {
        match self {
            Self::Resistance(_) => Unit::Ohm,
            Self::Voltage(_) => Unit::Volt,
            Self::Current(_) => Unit::Ampere,
            Self::Time(_) => Unit::Second,
        }
    }

    pub fn literal(token: &str, expected: Option<Unit>) -> Result<Self, String> {
        for unit in [Unit::Ohm, Unit::Volt, Unit::Ampere, Unit::Second] {
            let symbol = unit.symbol();
            if let Some((number, suffix)) =
                token.split_at_checked(token.len().saturating_sub(symbol.len()))
                && (suffix == symbol || unit != Unit::Second && suffix.eq_ignore_ascii_case(symbol))
            {
                if let Some(want) = expected
                    && want != unit
                {
                    return Err(format!("expected {}, got {}", want.symbol(), unit.symbol()));
                }
                return unit.value(number);
            }
        }
        expected
            .ok_or_else(|| format!("parameter default '{token}' needs a unit (Ohm, V, A or s)"))?
            .value(token)
    }
}
