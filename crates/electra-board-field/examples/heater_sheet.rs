//! Run a closed-form check for the steady thin-sheet thermal model.
#[path = "heater_coupon/model.rs"]
mod model;

use aequitas::systems::si::quantities::{
    Length, Power, TemperatureDifference, ThermalConductivity,
};
use model::{Comparison, Heat, HeatTransferCoefficient, Layer, Mesh, sample, solve_mesh};

fn main() {
    let mesh = Mesh {
        width: Length::from_base(3.),
        height: Length::from_base(1.),
        rows: 1,
        columns: 3,
        air: HeatTransferCoefficient::from_base(1.),
        layers: vec![Layer {
            thickness: Length::from_base(1.),
            conductivity: ThermalConductivity::from_base(1.),
        }],
        heat: vec![Heat {
            row: 0,
            column: 1,
            power: Power::from_base(1.),
        }],
    };
    let expected = [0.25, 0.5, 0.25];
    match solve_mesh(&mesh) {
        Ok(field)
            if field.rises.len() == expected.len()
                && field.rises.iter().zip(expected).all(|(actual, expected)| {
                    field.compare(
                        *actual - TemperatureDifference::from_base(expected),
                        field.solver_error + field.solver_error,
                    ) == Comparison::Within
                })
                && sample(&field, Length::from_base(1.5), Length::from_base(0.5)).is_ok_and(
                    |value| {
                        field.compare(
                            value - TemperatureDifference::from_base(0.5),
                            field.solver_error + field.solver_error,
                        ) == Comparison::Within
                    },
                )
                && field.sweeps > 0 =>
        {
            println!(
                "centre-heater check: [0.25, 0.5, 0.25] K; {} sweeps; {:.3e} W residual; {:.3e} K solver bound",
                field.sweeps,
                field.residual.into_base(),
                field.solver_error.into_base()
            )
        }
        Ok(field) => {
            eprintln!("analytical check mismatch: {:?}", field.rises);
            std::process::exit(1);
        }
        Err(error) => {
            eprintln!("analytical check failed: {error}");
            std::process::exit(1);
        }
    }
}
