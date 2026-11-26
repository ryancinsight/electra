//! Prescribed phasors for ground-referenced sources.
use super::{ParseError, parse::value};
use electra_ac::{Complex64, ElectricPotential};
use std::str::SplitWhitespace;

pub(super) fn prescribe(
    a: usize,
    b: usize,
    fields: &mut SplitWhitespace<'_>,
    line: usize,
    boundary: &mut [Option<ElectricPotential<Complex64>>],
) -> Result<(), ParseError> {
    let error = |s| ParseError::new(line, s);
    if !fields.next().is_some_and(|t| t.eq_ignore_ascii_case("AC")) {
        return Err(error("expected AC magnitude [phase_degrees]"));
    }
    let amplitude = value(fields.next(), line)?;
    if amplitude < 0. {
        return Err(error("negative AC magnitude"));
    }
    let phase = fields
        .next()
        .map(|t| value(Some(t), line))
        .transpose()?
        .unwrap_or(0.);
    let (target, sign) = match (a, b) {
        (0, n) if n != 0 => (n, -1.),
        (n, 0) if n != 0 => (n, 1.),
        _ => return Err(error("voltage source requires exactly one ground endpoint")),
    };
    if boundary[target].is_some() {
        return Err(error("duplicate prescribed node voltage"));
    }
    let (sin, cos) = match phase % 360. {
        0. => (0., 1.),
        90. | -270. => (1., 0.),
        -90. | 270. => (-1., 0.),
        180. | -180. => (0., -1.),
        degrees => degrees.to_radians().sin_cos(),
    };
    boundary[target] = Some(ElectricPotential::from_base(Complex64::new(
        sign * amplitude * cos,
        sign * amplitude * sin,
    )));
    Ok(())
}
