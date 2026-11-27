//! Startup from zero energy, followed by streaming mean and peak-to-peak readback.
use electra_circuit::{
    Dimensionless, ElectricCurrent, ElectricPotential, ElectricalImpedance,
    semiconductor::{JunctionDiode, ThermodynamicTemperature},
    transient::{
        Capacitance, Inductance, RlcState, Time,
        power::{HalfWaveRectifier, PowerError, SynchronousBuck},
    },
};

fn compare(samples: impl Iterator<Item = f64>, mean_ref: f64, ripple_ref: f64) {
    let (mut sum, mut low, mut high, mut count) = (0., f64::INFINITY, f64::NEG_INFINITY, 0);
    for v in samples {
        sum += v;
        low = low.min(v);
        high = high.max(v);
        count += 1;
    }
    let mean = sum / f64::from(count);
    let ripple = high - low;
    println!("mean {mean} ripple {ripple}");
    assert!((mean / mean_ref - 1.).abs() < 0.02 && (ripple / ripple_ref - 1.).abs() < 0.02);
}
fn rectifier() -> HalfWaveRectifier {
    let diode = JunctionDiode::new(
        ElectricCurrent::from_base(1e-12),
        Dimensionless::from_base(1.),
        ThermodynamicTemperature::from_base(300.15),
    )
    .unwrap();
    HalfWaveRectifier::new(
        diode,
        Capacitance::from_base(10e-6),
        ElectricalImpedance::from_base(100.),
    )
    .unwrap()
}
#[test]
fn half_wave_matches_ngspice_final_cycle() {
    let stage = rectifier();
    let mut state = ElectricPotential::default();
    let samples = (1..=100000).map(|k| {
        let vs = 2. * (std::f64::consts::TAU * 1e3 * f64::from(k) * 1e-7).sin();
        state = stage
            .advance(
                state,
                ElectricPotential::from_base(vs),
                Time::from_base(1e-7),
                ElectricCurrent::from_base(1e-12),
                64,
            )
            .unwrap();
        state.into_base()
    });
    compare(samples.skip(90000), 0.9807686, 0.7770946);
}
#[test]
fn synchronous_buck_matches_ngspice_final_ten_cycles() {
    let stage = SynchronousBuck::new(
        Inductance::from_base(100e-6),
        Capacitance::from_base(10e-6),
        ElectricalImpedance::from_base(5.),
    )
    .unwrap();
    let mut state = RlcState {
        voltage: ElectricPotential::default(),
        current: ElectricCurrent::default(),
    };
    let samples = (0..20000).map(|k| {
        state = stage
            .advance(
                state,
                ElectricPotential::from_base(5.),
                k % 1000 < 400,
                Time::from_base(1e-7),
            )
            .unwrap();
        state.voltage.into_base()
    });
    compare(samples.skip(10000), 1.999998, 1.876973);
}
#[test]
fn exhausted_step_and_newton_limit_are_distinct() {
    let stage = rectifier();
    let trial = |dt| {
        stage.advance(
            ElectricPotential::default(),
            ElectricPotential::from_base(2.),
            Time::from_base(dt),
            ElectricCurrent::from_base(1e-12),
            1,
        )
    };
    assert_eq!(trial(0.), Err(PowerError::TimestepExhausted));
    assert_eq!(trial(1e-6), Err(PowerError::NewtonLimit));
}
