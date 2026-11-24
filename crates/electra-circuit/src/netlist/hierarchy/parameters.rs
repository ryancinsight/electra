use super::super::value::{Unit, Value};

pub(super) struct Parameter<'a> {
    pub name: &'a str,
    pub default: Value,
}

pub(super) fn identifier(name: &str) -> Result<(), String> {
    if name.is_empty() || name.contains(['/', '{', '}', '=']) {
        Err(format!("invalid hierarchy identifier '{name}'"))
    } else {
        Ok(())
    }
}

pub(super) fn assignment(token: &str) -> Result<(&str, &str), String> {
    let (key, value) = token
        .split_once('=')
        .ok_or_else(|| format!("expected name=value, got '{token}'"))?;
    identifier(key)?;
    if value.is_empty() {
        return Err(format!("{key}: missing value"));
    }
    Ok((key, value))
}

pub(super) fn declare<'a>(params: &mut Vec<Parameter<'a>>, token: &'a str) -> Result<(), String> {
    let (name, token) = assignment(token)?;
    if params.iter().any(|p| p.name.eq_ignore_ascii_case(name)) {
        return Err(format!("duplicate parameter {name}"));
    }
    let default = Value::literal(token, None).map_err(|e| format!("{name}: {e}"))?;
    params.push(Parameter { name, default });
    Ok(())
}

#[derive(Copy, Clone)]
pub(super) enum Operand {
    Constant(Value),
    Parameter(usize),
}
impl Operand {
    pub fn bind(
        token: &str,
        want: Unit,
        params: &[Parameter],
        unit_literal: bool,
    ) -> Result<Self, String> {
        if let Some(key) = token.strip_prefix('{').and_then(|t| t.strip_suffix('}')) {
            let i = params
                .iter()
                .position(|p| p.name.eq_ignore_ascii_case(key))
                .ok_or_else(|| format!("unknown local parameter {key}"))?;
            let unit = params[i].default.unit();
            if unit != want {
                return Err(format!(
                    "parameter {key}: expected {}, got {}",
                    want.symbol(),
                    unit.symbol()
                ));
            }
            Ok(Self::Parameter(i))
        } else {
            (if unit_literal {
                Value::literal(token, Some(want))
            } else {
                want.value(token)
            })
            .map(Self::Constant)
        }
    }
    pub fn get(self, env: &[Value]) -> Value {
        match self {
            Self::Constant(v) => v,
            Self::Parameter(i) => env[i],
        }
    }
}
