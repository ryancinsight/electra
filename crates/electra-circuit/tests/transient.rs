//! Analytical qualification of first-order series circuit transients.
use electra_circuit::{
    ElectricCurrent, ElectricPotential, ElectricalImpedance,
    transient::{Capacitance, Inductance, SeriesRc, SeriesRl, StepError, Time},
};

#[test]
fn rc_and_rl_steps_converge_at_first_order() {
    let r = ElectricalImpedance::from_base(4.);
    let rc = SeriesRc::new(r, Capacitance::from_base(0.125)).unwrap();
    let rl = SeriesRl::new(r, Inductance::from_base(2.)).unwrap();
    let drive = ElectricPotential::from_base(-8.);
    let exact = -(-1.0_f64).exp_m1();
    let mut previous = None;
    for n in [64, 128, 256] {
        let dt = Time::from_base(0.5 / f64::from(n));
        let mut v = ElectricPotential::default();
        let mut i = ElectricCurrent::default();
        for _ in 0..n {
            v = rc.advance(v, drive, dt).unwrap();
            i = rl.advance(i, drive, dt).unwrap();
        }
        let error = (v.into_base() / -8. - exact).abs();
        assert!((i.into_base() / -2. - exact).abs() <= error + 1e-14);
        if let Some(prev) = previous {
            assert!((1.95..2.05).contains(&(prev / error)));
        }
        if n == 256 {
            assert!(error / exact < 0.002);
        }
        previous = Some(error);
    }
}

#[test]
fn zero_step_preserves_state_and_large_times_average_without_overflow() {
    let rc = SeriesRc::new(
        ElectricalImpedance::from_base(f64::MAX),
        Capacitance::from_base(1.),
    )
    .unwrap();
    let state = ElectricPotential::from_base(1.);
    let drive = ElectricPotential::from_base(-3.);
    assert_eq!(rc.advance(state, drive, Time::default()).unwrap(), state);
    assert_eq!(
        rc.advance(state, drive, Time::from_base(f64::MAX))
            .unwrap()
            .into_base(),
        -1.
    );
    assert_eq!(
        rc.advance(state, drive, Time::from_base(-1.)).unwrap_err(),
        StepError::Interval
    );
    assert_eq!(
        rc.advance(
            state,
            ElectricPotential::from_base(f64::NAN),
            Time::from_base(1.)
        )
        .unwrap_err(),
        StepError::State
    );
    assert!(
        SeriesRl::new(
            ElectricalImpedance::from_base(-1.),
            Inductance::from_base(-1.)
        )
        .is_err()
    );
}
