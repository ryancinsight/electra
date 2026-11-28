//! Run a closed-form check for the steady thin-sheet thermal model.
#[path = "heater_coupon/model.rs"]
mod model;

use aequitas::systems::si::quantities::{Length, Power, ThermalConductivity};
use model::{Heat, HeatTransferCoefficient, Layer, Mesh, sample, solve_mesh};

const ANALYTICAL_BOUND_K: f64 = 2.1e-10;

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
                    (actual.into_base() - expected).abs() <= ANALYTICAL_BOUND_K
                })
                && sample(&field, Length::from_base(1.5), Length::from_base(0.5))
                    .is_ok_and(|value| (value.into_base() - 0.5).abs() <= ANALYTICAL_BOUND_K)
                && field.sweeps > 0
                && field.residual.into_base() <= 2.1e-10 =>
        {
            println!(
                "centre-heater check: [0.25, 0.5, 0.25] K; {} sweeps; {:.3e} W residual",
                field.sweeps,
                field.residual.into_base()
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
