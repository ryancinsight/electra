use aequitas::systems::si::quantities::{
    Area, Length, Power, ReciprocalTemperature, TemperatureDifference, ThermalConductivity,
};
use electra_network::{Branch, solve};

const MAX_CELLS: usize = 4_096;
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
}

pub(crate) fn solve_mesh(mesh: &Mesh) -> Result<Field, String> {
    validate(mesh)?;
    let dx = mesh.width / scalar(mesh.columns)?;
    let dy = mesh.height / scalar(mesh.rows)?;
    let mut sheet = Conductance::default();
    for layer in &mesh.layers {
        sheet += layer.conductivity * layer.thickness;
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
            let here = node(row, column, mesh.columns);
            branches.push(Branch {
                from: here,
                to: 0,
                conductance: mesh.air * dx * dy,
            });
            if column + 1 < mesh.columns {
                branches.push(Branch {
                    from: here,
                    to: node(row, column + 1, mesh.columns),
                    conductance: sheet * dy / dx,
                });
            }
            if row + 1 < mesh.rows {
                branches.push(Branch {
                    from: here,
                    to: node(row + 1, column, mesh.columns),
                    conductance: sheet * dx / dy,
                });
            }
        }
    }
    let point = solve(&fixed, &powers, &branches, Default::default())
        .map_err(|error| format!("thermal network: {error}"))?;
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
    })
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

    // The inverse maximum norm is 1 K/W. The solver's 1e-12 W absolute and
    // 1e-10 relative residual bounds give less than 2.1e-10 K here.
    const ANALYTICAL_BOUND_K: f64 = 2.1e-10;

    fn solve_line(columns: usize, powers: &[f64]) -> Result<Field, String> {
        solve_mesh(&Mesh {
            width: Length::from_base(scalar(columns)?),
            height: Length::from_base(1.),
            rows: 1,
            columns,
            air: HeatTransferCoefficient::from_base(1.),
            layers: vec![Layer {
                thickness: Length::from_base(1.),
                conductivity: ThermalConductivity::from_base(1.),
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
        close(&solve_line(2, &[0.5, 0.5]).unwrap().rises, &[0.5, 0.5]);
        close(
            &solve_line(3, &[0., 1., 0.]).unwrap().rises,
            &[0.25, 0.5, 0.25],
        );
        let edge = solve_line(3, &[1., 0., 0.]).unwrap();
        close(&edge.rises, &[0.625, 0.25, 0.125]);
        let midpoint = sample(&edge, Length::from_base(1.), Length::from_base(0.5)).unwrap();
        assert!((midpoint.into_base() - 0.4375).abs() <= ANALYTICAL_BOUND_K);
        close(&solve_line(2, &[0., 0.]).unwrap().rises, &[0., 0.]);
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
