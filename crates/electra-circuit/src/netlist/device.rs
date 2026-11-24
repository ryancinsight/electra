//! One card grammar for flat reading and subcircuit linking.
use super::value::Unit;

#[derive(Clone, Copy)]
pub(super) enum Input<N, B> {
    Independent,
    Potential([N; 2]),
    Branch(B),
}

impl<N, B> Input<N, B> {
    pub fn map<M, C, E>(
        self,
        mut node: impl FnMut(N) -> Result<M, E>,
        branch: impl FnOnce(B) -> Result<C, E>,
    ) -> Result<Input<M, C>, E> {
        Ok(match self {
            Self::Independent => Input::Independent,
            Self::Potential([a, b]) => Input::Potential([node(a)?, node(b)?]),
            Self::Branch(b) => Input::Branch(branch(b)?),
        })
    }
}

pub(super) struct Device<'a> {
    pub ports: [&'a str; 2],
    pub input: Input<&'a str, &'a str>,
    pub unit: Unit,
    pub magnitude: &'a str,
}

impl<'a> Device<'a> {
    pub fn read(name: &str, mut words: std::str::SplitWhitespace<'a>) -> Result<Self, String> {
        let unit = Unit::device(name)?;
        let kind = name.as_bytes()[0].to_ascii_uppercase();
        let missing = || {
            format!(
                "{name}: expected output terminals, control operands for the source type, and one value"
            )
        };
        let ports = [
            words.next().ok_or_else(missing)?,
            words.next().ok_or_else(missing)?,
        ];
        let input = match kind {
            b'E' | b'G' => Input::Potential([
                words.next().ok_or_else(missing)?,
                words.next().ok_or_else(missing)?,
            ]),
            b'F' | b'H' => Input::Branch(words.next().ok_or_else(missing)?),
            _ => Input::Independent,
        };
        let magnitude = words.next().ok_or_else(missing)?;
        if words.next().is_some() {
            return Err(format!("{name}: unexpected trailing fields"));
        }
        Ok(Self {
            ports,
            input,
            unit,
            magnitude,
        })
    }
}
