//! Independent sinusoidal reference and rejected-trial replay for the adaptive solver.
use electra_circuit::{
    ElectricCurrent, ElectricPotential, ElectricalImpedance,
    transient::{AdaptiveOptions, Capacitance, Inductance, RlcState, SeriesRlc, Time},
};

fn oscillator() -> SeriesRlc {
    SeriesRlc::new(
        ElectricalImpedance::default(),
        Inductance::from_base(1.),
        Capacitance::from_base(1.),
    )
    .unwrap()
}
fn zero() -> RlcState {
    RlcState {
        current: Default::default(),
        voltage: Default::default(),
    }
}
fn options(h: f64, tol: f64) -> AdaptiveOptions {
    AdaptiveOptions {
        maximum_step: Time::from_base(h),
        current_error: ElectricCurrent::from_base(tol),
        voltage_error: ElectricPotential::from_base(tol),
    }
}
#[test]
fn rejected_trial_replays_from_initial_state() {
    let m = oscillator();
    let v = ElectricPotential::from_base(1.);
    let result = m
        .advance_segment(zero(), v, Time::from_base(1.), &options(1., 0.01))
        .unwrap();
    assert_eq!((result.accepted, result.rejected), (2, 1));
    let mut expected = zero();
    for _ in 0..4 {
        expected = m.advance(expected, v, Time::from_base(0.25)).unwrap();
    }
    assert_eq!(result.state, expected);
}
#[test]
fn source_event_and_refinement_match_piecewise_exact_solution() {
    let m = oscillator();
    let switch = 0.73_f64;
    let tail = 0.36_f64;
    let v = 1. - switch.cos();
    let i = switch.sin();
    let exact_voltage = v * tail.cos() + i * tail.sin();
    let exact_current = i * tail.cos() - v * tail.sin();
    let mut previous = None;
    for h in [0.04, 0.02, 0.01] {
        let control = options(h, 1e-5);
        let first = m
            .advance_segment(
                zero(),
                ElectricPotential::from_base(1.),
                Time::from_base(switch),
                &control,
            )
            .unwrap();
        assert_eq!(first.accepted, (switch / h).ceil() as usize);
        // For this lossless driven oscillator, atan2(i, 1-v) reads back elapsed
        // time in seconds during the first segment (angular frequency 1 rad/s).
        let event_time = first
            .state
            .current
            .into_base()
            .atan2(1. - first.state.voltage.into_base());
        assert!((event_time - switch).abs() < 3e-5);
        let end = m
            .advance_segment(
                first.state,
                ElectricPotential::default(),
                Time::from_base(tail),
                &control,
            )
            .unwrap();
        let error = (end.state.voltage.into_base() - exact_voltage)
            .hypot(end.state.current.into_base() - exact_current);
        if let Some(old) = previous {
            assert!((3.8..4.2).contains(&(old / error)));
        }
        previous = Some(error);
        println!("maximum step {h}: endpoint error {error:e}");
    }
}
