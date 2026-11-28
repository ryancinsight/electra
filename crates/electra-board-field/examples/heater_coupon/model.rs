use aequitas::systems::si::quantities::{
    Area, Length, Power, ReciprocalTemperature, TemperatureDifference, ThermalConductivity,
};
use electra_network::{Branch, SolveOptions, solve};
use std::cmp::Ordering;

const MAX_CELLS: usize = 4_096;
const MAX_ROW_BRANCHES: u32 = 5;
const UNIT_ROUNDOFF: f64 = f64::EPSILON / 2.;
const MIN_SUBNORMAL: f64 = f64::from_bits(1);
type Conductance = <Power as std::ops::Mul<ReciprocalTemperature>>::Output;
pub(crate) type HeatTransferCoefficient = <Conductance as std::ops::Div<Area>>::Output;
type NetworkTemperature = <Power as std::ops::Div<Conductance>>::Output;

#[derive(Clone, Copy)]
pub(crate) struct Layer {
    pub(crate) thickness: Length,
    pub(crate) conductivity: ThermalConductivity,
}

#[derive(Clone, Copy)]
pub(crate) struct Heat {
    pub(crate) row: usize,
    pub(crate) column: usize,
    pub(crate) power: Power,
}

pub(crate) struct Mesh {
    pub(crate) width: Length,
    pub(crate) height: Length,
    pub(crate) rows: usize,
    pub(crate) columns: usize,
    pub(crate) air: HeatTransferCoefficient,
    pub(crate) layers: Vec<Layer>,
    pub(crate) heat: Vec<Heat>,
}

#[derive(Debug)]
pub(crate) struct Field {
    pub(crate) rises: Vec<TemperatureDifference>,
    pub(crate) x_centres: Vec<Length>,
    pub(crate) y_centres: Vec<Length>,
    width: Length,
    height: Length,
    pub(crate) sweeps: usize,
    pub(crate) residual: Power,
    pub(crate) solver_error: TemperatureDifference,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Comparison {
    Within,
    Outside,
    Unresolved,
}

impl Field {
    pub(crate) fn compare(
        &self,
        residual: TemperatureDifference,
        interval: TemperatureDifference,
    ) -> Comparison {
        compare(residual, interval, self.solver_error)
    }
}

fn compare(
    residual: TemperatureDifference,
    interval: TemperatureDifference,
    solver_error: TemperatureDifference,
) -> Comparison {
    let distance = residual.into_base().abs();
    let interval = interval.into_base();
    let error = solver_error.into_base();
    if !distance.is_finite()
        || !interval.is_finite()
        || interval < 0.
        || !error.is_finite()
        || error < 0.
    {
        return Comparison::Unresolved;
    }
    match exact_sum_cmp(distance, error, interval) {
        Some(Ordering::Less | Ordering::Equal) => Comparison::Within,
        Some(Ordering::Greater) => match exact_sum_cmp(interval, error, distance) {
            Some(Ordering::Less) => Comparison::Outside,
            Some(Ordering::Equal | Ordering::Greater) | None => Comparison::Unresolved,
        },
        None => Comparison::Unresolved,
    }
}

fn exact_sum_cmp(left: f64, right: f64, limit: f64) -> Option<Ordering> {
    if !left.is_finite() || left < 0. || !right.is_finite() || right < 0. || !limit.is_finite() {
        return None;
    }
    let (larger, smaller) = if left >= right {
        (left, right)
    } else {
        (right, left)
    };
    let sum = larger + smaller;
    if sum.is_infinite() {
        return Some(Ordering::Greater);
    }
    let low = smaller - (sum - larger);
    match sum.total_cmp(&limit) {
        Ordering::Equal if low < 0. => Some(Ordering::Less),
        Ordering::Equal if low > 0. => Some(Ordering::Greater),
        order => Some(order),
    }
}

pub(crate) fn solve_mesh(mesh: &Mesh) -> Result<Field, String> {
    validate(mesh)?;
    let dx = mesh.width / scalar(mesh.columns)?;
    let dy = mesh.height / scalar(mesh.rows)?;
    let mut sheet = Conductance::default();
    for layer in &mesh.layers {
        sheet += layer.conductivity * layer.thickness;
    }
    let sink = mesh.air * dx * dy;
    if !sink.into_base().is_finite() || sink <= Conductance::default() {
        return Err("thermal sink conductance is not representable".into());
    }
    let horizontal = (mesh.columns > 1).then(|| sheet * dy / dx);
    let vertical = (mesh.rows > 1).then(|| sheet * dx / dy);
    for conductance in [horizontal, vertical].into_iter().flatten() {
        if !conductance.into_base().is_finite() || conductance <= Conductance::default() {
            return Err("thermal sheet conductance is not representable".into());
        }
    }
    let cells = mesh.rows * mesh.columns;
    let mut fixed = vec![None; cells + 1];
    fixed[0] = Some(NetworkTemperature::default());
    let mut powers = vec![Power::default(); cells + 1];
    for heat in &mesh.heat {
        powers[node(heat.row, heat.column, mesh.columns)] += heat.power;
    }
    let mut branches = Vec::with_capacity(cells * 3);
    for row in 0..mesh.rows {
        for column in 0..mesh.columns {
            let cell = row * mesh.columns + column;
            let here = cell + 1;
            branches.push(Branch {
                from: here,
                to: 0,
                conductance: sink,
            });
            if column + 1 < mesh.columns {
                let other_cell = cell + 1;
                let conductance = horizontal
                    .expect("invariant: a horizontal edge requires multiple mesh columns");
                branches.push(Branch {
                    from: here,
                    to: other_cell + 1,
                    conductance,
                });
            }
            if row + 1 < mesh.rows {
                let other_cell = cell + mesh.columns;
                let conductance =
                    vertical.expect("invariant: a vertical edge requires multiple mesh rows");
                branches.push(Branch {
                    from: here,
                    to: other_cell + 1,
                    conductance,
                });
            }
        }
    }
    let options = SolveOptions {
        absolute: Power::default(),
        ..Default::default()
    };
    let point = solve(&fixed, &powers, &branches, options)
        .map_err(|error| format!("thermal network: {error}"))?;
    let maximum_temperature = point
        .potentials
        .iter()
        .skip(1)
        .map(|value| value.into_base().abs())
        .fold(0_f64, f64::max);
    let maximum_injection = powers
        .iter()
        .skip(1)
        .map(|power| power.into_base().abs())
        .fold(0_f64, f64::max);
    let horizontal_count = (mesh.columns - 1).min(2);
    let vertical_count = (mesh.rows - 1).min(2);
    let mut maximum_conductance = sink.into_base();
    if let Some(conductance) = horizontal {
        maximum_conductance = upward_add(
            maximum_conductance,
            upward_mul(scalar(horizontal_count)?, conductance.into_base())?,
        )?;
    }
    if let Some(conductance) = vertical {
        maximum_conductance = upward_add(
            maximum_conductance,
            upward_mul(scalar(vertical_count)?, conductance.into_base())?,
        )?;
    }
    let branch_count = u32::try_from(1 + horizontal_count + vertical_count)
        .map_err(|_| "thermal row branch count cannot be represented")?;
    let roundoff = row_roundoff(
        maximum_injection,
        maximum_temperature,
        maximum_conductance,
        branch_count,
    )?;
    let residual_bound = upward_add(point.max_residual.into_base(), roundoff)?;
    let solver_error = upward_div(residual_bound, sink.into_base())?;
    Ok(Field {
        rises: point
            .potentials
            .into_iter()
            .skip(1)
            .map(|value| TemperatureDifference::from_base(value.into_base()))
            .collect(),
        x_centres: centres(mesh.columns, dx)?,
        y_centres: centres(mesh.rows, dy)?,
        width: mesh.width,
        height: mesh.height,
        sweeps: point.sweeps,
        residual: point.max_residual,
        solver_error: TemperatureDifference::from_base(solver_error),
    })
}

fn row_roundoff(
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

fn upward_add(left: f64, right: f64) -> Result<f64, String> {
    next_up(left + right)
}

fn upward_mul(left: f64, right: f64) -> Result<f64, String> {
    next_up(left * right)
}

fn upward_div(numerator: f64, denominator: f64) -> Result<f64, String> {
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

pub(crate) fn sample(field: &Field, x: Length, y: Length) -> Result<TemperatureDifference, String> {
    if !x.into_base().is_finite()
        || !y.into_base().is_finite()
        || x < Length::default()
        || x > field.width
        || y < Length::default()
        || y > field.height
        || field.x_centres.is_empty()
        || field.y_centres.is_empty()
        || field.rises.len() != field.x_centres.len() * field.y_centres.len()
    {
        return Err("invalid sample coordinate or field shape".into());
    }
    let (left, x_fraction) = bracket(&field.x_centres, x);
    let (top, y_fraction) = bracket(&field.y_centres, y);
    let columns = field.x_centres.len();
    let right = (left + 1).min(columns - 1);
    let bottom = (top + 1).min(field.y_centres.len() - 1);
    let get = |row: usize, column: usize| field.rises[row * columns + column].into_base();
    let upper = get(top, left) * (1. - x_fraction) + get(top, right) * x_fraction;
    let lower = get(bottom, left) * (1. - x_fraction) + get(bottom, right) * x_fraction;
    Ok(TemperatureDifference::from_base(
        upper * (1. - y_fraction) + lower * y_fraction,
    ))
}

fn validate(mesh: &Mesh) -> Result<(), String> {
    let positive = |value: f64| value.is_finite() && value > 0.;
    let cells = mesh
        .rows
        .checked_mul(mesh.columns)
        .filter(|count| (1..=MAX_CELLS).contains(count));
    if cells.is_none()
        || ![
            mesh.width.into_base(),
            mesh.height.into_base(),
            mesh.air.into_base(),
        ]
        .into_iter()
        .all(positive)
        || mesh.layers.is_empty()
        || mesh.layers.iter().any(|layer| {
            !positive(layer.thickness.into_base()) || !positive(layer.conductivity.into_base())
        })
        || mesh.heat.iter().any(|heat| {
            heat.row >= mesh.rows
                || heat.column >= mesh.columns
                || !heat.power.into_base().is_finite()
                || heat.power.into_base() < 0.
        })
    {
        return Err("invalid mesh geometry, material, air conductance, or heat source".into());
    }
    Ok(())
}

fn scalar(value: usize) -> Result<f64, String> {
    u32::try_from(value)
        .map(f64::from)
        .map_err(|_| "mesh count cannot be represented exactly".into())
}

fn centres(count: usize, step: Length) -> Result<Vec<Length>, String> {
    (0..count)
        .map(|index| Ok((scalar(index)? + 0.5) * step))
        .collect()
}

fn node(row: usize, column: usize, columns: usize) -> usize {
    row * columns + column + 1
}

fn bracket(centres: &[Length], value: Length) -> (usize, f64) {
    let upper = centres.partition_point(|centre| *centre <= value);
    if upper == 0 || centres.len() == 1 {
        return (0, 0.);
    }
    if upper == centres.len() {
        return (upper - 1, 0.);
    }
    let lower = upper - 1;
    (
        lower,
        ((value - centres[lower]) / (centres[upper] - centres[lower])).into_base(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // The inverse maximum norm is 1 K/W for unit sink conductance. The
    // scale-invariant 1e-10 relative residual gives less than 2e-10 K here.
    const ANALYTICAL_BOUND_K: f64 = 2.1e-10;

    fn solve_line(columns: usize, powers: &[f64], conductance: f64) -> Result<Field, String> {
        solve_mesh(&Mesh {
            width: Length::from_base(scalar(columns)?),
            height: Length::from_base(1.),
            rows: 1,
            columns,
            air: HeatTransferCoefficient::from_base(conductance),
            layers: vec![Layer {
                thickness: Length::from_base(1.),
                conductivity: ThermalConductivity::from_base(conductance),
            }],
            heat: powers
                .iter()
                .enumerate()
                .map(|(column, power)| Heat {
                    row: 0,
                    column,
                    power: Power::from_base(*power),
                })
                .collect(),
        })
    }

    fn close(actual: &[TemperatureDifference], expected: &[f64]) {
        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.iter().zip(expected) {
            assert!((actual.into_base() - expected).abs() <= ANALYTICAL_BOUND_K);
        }
    }

    #[test]
    fn uniform_centre_edge_and_zero_loads_match_closed_form() {
        close(&solve_line(2, &[0.5, 0.5], 1.).unwrap().rises, &[0.5, 0.5]);
        close(
            &solve_line(3, &[0., 1., 0.], 1.).unwrap().rises,
            &[0.25, 0.5, 0.25],
        );
        let edge = solve_line(3, &[1., 0., 0.], 1.).unwrap();
        close(&edge.rises, &[0.625, 0.25, 0.125]);
        let midpoint = sample(&edge, Length::from_base(1.), Length::from_base(0.5)).unwrap();
        assert!((midpoint.into_base() - 0.4375).abs() <= ANALYTICAL_BOUND_K);
        close(&solve_line(2, &[0., 0.], 1.).unwrap().rises, &[0., 0.]);
    }

    #[test]
    fn scaled_edge_load_accepts_exact_and_rejects_false_reference() {
        let field = solve_line(3, &[1e-13, 0., 0.], 1e-13).unwrap();
        let expected = [0.625, 0.25, 0.125];
        assert!(field.sweeps > 1);
        assert_eq!(field.rises.len(), expected.len());
        for (actual, expected) in field.rises.iter().zip(expected) {
            assert!((actual.into_base() - expected).abs() <= field.solver_error.into_base());
        }
        let interval = TemperatureDifference::from_base(1e-6);
        assert_eq!(
            field.compare(
                field.rises[0] - TemperatureDifference::from_base(0.625),
                interval,
            ),
            Comparison::Within
        );
        assert_eq!(
            field.compare(
                field.rises[0] - TemperatureDifference::from_base(0.5),
                interval,
            ),
            Comparison::Outside
        );
    }

    #[test]
    fn comparison_boundary_requires_complete_containment() {
        let interval = TemperatureDifference::from_base(0.5);
        let error = TemperatureDifference::from_base(0.125);
        assert_eq!(
            compare(TemperatureDifference::from_base(0.375), interval, error),
            Comparison::Within
        );
        let next = f64::from_bits(0.375_f64.to_bits() + 1);
        assert_eq!(
            compare(TemperatureDifference::from_base(next), interval, error),
            Comparison::Unresolved
        );
        assert_eq!(
            compare(TemperatureDifference::from_base(0.75), interval, error),
            Comparison::Outside
        );
        let below_half_ulp = TemperatureDifference::from_base(f64::EPSILON / 4.);
        assert_eq!(
            compare(
                TemperatureDifference::from_base(1.),
                TemperatureDifference::from_base(1.),
                below_half_ulp,
            ),
            Comparison::Unresolved
        );
        assert_eq!(
            compare(TemperatureDifference::from_base(f64::NAN), interval, error,),
            Comparison::Unresolved
        );
        assert_eq!(
            compare(
                TemperatureDifference::default(),
                TemperatureDifference::from_base(f64::INFINITY),
                error,
            ),
            Comparison::Unresolved
        );
        assert_eq!(
            compare(
                TemperatureDifference::default(),
                interval,
                TemperatureDifference::from_base(f64::INFINITY),
            ),
            Comparison::Unresolved
        );
    }

    #[test]
    fn rounded_zero_residual_has_a_nonzero_error_certificate() {
        let field = solve_line(1, &[1.], 3.).unwrap();
        assert_eq!(field.rises, [TemperatureDifference::from_base(1. / 3.)]);
        assert_eq!(field.residual, Power::default());
        let exact_error = (f64::EPSILON / 4.) / 3.;
        assert!(field.solver_error.into_base() >= exact_error);
    }

    #[test]
    fn rejects_invalid_geometry_and_source_registration() {
        let mut mesh = Mesh {
            width: Length::from_base(1.),
            height: Length::from_base(1.),
            rows: 1,
            columns: 1,
            air: HeatTransferCoefficient::from_base(1.),
            layers: vec![Layer {
                thickness: Length::from_base(1.),
                conductivity: ThermalConductivity::from_base(1.),
            }],
            heat: vec![],
        };
        let field = solve_mesh(&mesh).unwrap();
        assert_eq!(
            sample(&field, Length::from_base(f64::NAN), Length::default()).unwrap_err(),
            "invalid sample coordinate or field shape"
        );
        assert_eq!(
            sample(&field, Length::from_base(2.), Length::default()).unwrap_err(),
            "invalid sample coordinate or field shape"
        );
        mesh.width = Length::from_base(MIN_SUBNORMAL);
        mesh.air = HeatTransferCoefficient::from_base(MIN_SUBNORMAL);
        assert_eq!(
            solve_mesh(&mesh).unwrap_err(),
            "thermal sink conductance is not representable"
        );
        mesh.air = HeatTransferCoefficient::from_base(1.);
        mesh.width = Length::default();
        assert_eq!(
            solve_mesh(&mesh).unwrap_err(),
            "invalid mesh geometry, material, air conductance, or heat source"
        );
        mesh.width = Length::from_base(1.);
        mesh.heat.push(Heat {
            row: 1,
            column: 0,
            power: Power::from_base(1.),
        });
        assert_eq!(
            solve_mesh(&mesh).unwrap_err(),
            "invalid mesh geometry, material, air conductance, or heat source"
        );
    }
}
