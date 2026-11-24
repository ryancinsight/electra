//! Physical values at the deck boundary. Formal dimensions survive through binding.
use super::number::magnitude;
use crate::{
    Dimensionless, ElectricConductance, ElectricCurrent, ElectricPotential, ElectricalImpedance,
};
use aequitas::systems::si::quantities::Time;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Unit {
    Gain,
    Siemens,
    Ohm,
    Volt,
    Ampere,
    Second,
}

impl Unit {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Gain => "x",
            Self::Siemens => "S",
            Self::Ohm => "Ohm",
            Self::Volt => "V",
            Self::Ampere => "A",
            Self::Second => "s",
        }
    }
    pub fn device(name: &str) -> Result<Self, String> {
        match name.as_bytes().first().map(u8::to_ascii_uppercase) {
            Some(b'R' | b'H') => Ok(Self::Ohm),
            Some(b'E' | b'F') => Ok(Self::Gain),
            Some(b'G') => Ok(Self::Siemens),
            Some(b'V') => Ok(Self::Volt),
            Some(b'I') => Ok(Self::Ampere),
            _ => Err(format!("unsupported element {name}")),
        }
    }
    pub fn value(self, token: &str) -> Result<Value, String> {
        let value = magnitude(token)
            .ok_or_else(|| format!("invalid finite {} magnitude '{token}'", self.symbol()))?;
        Ok(match self {
            Self::Gain => Value::Gain(Dimensionless::from_base(value)),
            Self::Siemens => Value::Conductance(ElectricConductance::from_base(value)),
            Self::Ohm => Value::Resistance(ElectricalImpedance::from_base(value)),
            Self::Volt => Value::Voltage(ElectricPotential::from_base(value)),
            Self::Ampere => Value::Current(ElectricCurrent::from_base(value)),
            Self::Second => Value::Time(Time::from_base(value)),
        })
    }
}

#[derive(Clone, Copy)]
pub(super) enum Value {
    Gain(Dimensionless),
    Conductance(ElectricConductance),
    Resistance(ElectricalImpedance),
    Voltage(ElectricPotential),
    Current(ElectricCurrent),
    Time(Time),
}
impl Value {
    pub fn unit(self) -> Unit {
        match self {
            Self::Gain(_) => Unit::Gain,
            Self::Conductance(_) => Unit::Siemens,
            Self::Resistance(_) => Unit::Ohm,
            Self::Voltage(_) => Unit::Volt,
            Self::Current(_) => Unit::Ampere,
            Self::Time(_) => Unit::Second,
        }
    }

    pub fn literal(token: &str, expected: Option<Unit>) -> Result<Self, String> {
        for unit in [
            Unit::Ohm,
            Unit::Volt,
            Unit::Ampere,
            Unit::Second,
            Unit::Siemens,
            Unit::Gain,
        ] {
            let symbol = unit.symbol();
            if let Some((number, suffix)) =
                token.split_at_checked(token.len().saturating_sub(symbol.len()))
                && (suffix == symbol
                    || !matches!(unit, Unit::Second | Unit::Siemens)
                        && suffix.eq_ignore_ascii_case(symbol))
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
            .ok_or_else(|| {
                format!("parameter default '{token}' needs a unit (Ohm, V, A, s, S or x)")
            })?
            .value(token)
    }
}
