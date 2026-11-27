//! Independent ngspice DC sweep and numerical range regressions.
use electra_circuit::{
    Dimensionless, ElectricCurrent, ElectricPotential,
    semiconductor::{JunctionDiode, ThermodynamicTemperature},
};

fn diode(saturation: f64) -> JunctionDiode {
    JunctionDiode::new(
        ElectricCurrent::from_base(saturation),
        Dimensionless::from_base(1.5),
        ThermodynamicTemperature::from_base(300.15),
    )
    .unwrap()
}
#[test]
fn ngspice_nominal_temperature_iv_and_conductance() {
    let model = diode(1e-12);
    let mut max_current = 0.0_f64;
    let mut max_slope = 0.0_f64;
    let reference = include_str!("../../../examples/semiconductor/diode-iv.dat");
    for line in reference.lines() {
        let mut values = line.split_whitespace().map(|v| v.parse::<f64>().unwrap());
        let v = values.next().unwrap();
        let reference_i = values.next().unwrap();
        let reference_g = values.next().unwrap();
        let p = model.at(ElectricPotential::from_base(v)).unwrap();
        max_current = max_current
            .max((p.current.into_base() - reference_i).abs() / reference_i.abs().max(1e-15));
        max_slope = max_slope.max((p.conductance.into_base() / reference_g - 1.).abs());
    }
    assert_eq!(reference.lines().count(), 33);
    println!(
        "maximum floor-normalized current error {max_current:e}, relative conductance error {max_slope:e}"
    );
    assert!(max_current < 0.01 && max_slope < 0.01);
}
#[test]
fn cancellation_overflow_and_reverse_limit() {
    let model = diode(1e-300);
    let v = ElectricPotential::from_base;
    let p = model.at(v(28.)).unwrap();
    assert!(p.current.into_base() > 1. && p.conductance.into_base() > p.current.into_base());
    let model = diode(1e-12);
    let zero = model.at(v(0.)).unwrap();
    assert_eq!(zero.current.into_base(), 0.);
    let tiny = model.at(v(1e-20)).unwrap();
    assert!((tiny.current.into_base() / 1e-20 / zero.conductance.into_base() - 1.).abs() < 1e-14);
    assert_eq!(model.at(v(-100.)).unwrap().current.into_base(), -1e-12);
    assert!(model.at(v(100.)).is_none());
    assert!(model.at(v(f64::NAN)).is_none());
}
