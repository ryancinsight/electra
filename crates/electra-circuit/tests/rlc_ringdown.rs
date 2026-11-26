//! Measure free-decay peaks from simulated current crossings and capacitor voltage.
use electra_circuit::{
    ElectricPotential, ElectricalImpedance,
    transient::{Capacitance, Inductance, RlcState, SeriesRlc, Time},
};

#[test]
fn damped_period_and_log_decrement_converge_with_refinement() {
    let model = SeriesRlc::new(
        ElectricalImpedance::from_base(2.),
        Inductance::from_base(0.1),
        Capacitance::from_base(0.001),
    )
    .unwrap();
    let alpha = 10.;
    let omega = (10000.0_f64 - alpha * alpha).sqrt();
    let period = std::f64::consts::TAU / omega;
    let decrement = alpha * period;
    let mut prior = None;
    for dt in [0.001, 0.0005, 0.00025] {
        let mut state = RlcState {
            voltage: ElectricPotential::from_base(1.),
            current: Default::default(),
        };
        let mut peaks = Vec::new();
        for k in 0..1000 {
            let next = model
                .advance(state, ElectricPotential::default(), Time::from_base(dt))
                .unwrap();
            let i = state.current.into_base();
            let j = next.current.into_base();
            if i > 0. && j <= 0. {
                let fraction = i / (i - j);
                let voltage = state.voltage.into_base()
                    + fraction * (next.voltage.into_base() - state.voltage.into_base());
                peaks.push(((f64::from(k) + fraction) * dt, voltage));
                if peaks.len() == 2 {
                    break;
                }
            }
            state = next;
        }
        assert_eq!(peaks.len(), 2);
        let period_error = ((peaks[1].0 - peaks[0].0) / period - 1.).abs();
        let decay_error = ((peaks[0].1 / peaks[1].1).ln() / decrement - 1.).abs();
        assert!(period_error < 0.005 && decay_error < 0.005);
        if let Some(previous) = prior {
            assert!((3.0..5.0).contains(&(previous / period_error)));
        }
        prior = Some(period_error);
        println!("dt={dt} period_error={period_error:e} decrement_error={decay_error:e}");
    }
}

#[test]
fn lossless_driven_quarter_cycle_and_zero_step() {
    let model = SeriesRlc::new(
        ElectricalImpedance::default(),
        Inductance::from_base(1.),
        Capacitance::from_base(1.),
    )
    .unwrap();
    let drive = ElectricPotential::from_base(2.);
    let mut state = RlcState {
        voltage: Default::default(),
        current: Default::default(),
    };
    assert_eq!(model.advance(state, drive, Time::default()).unwrap(), state);
    for _ in 0..256 {
        state = model
            .advance(
                state,
                drive,
                Time::from_base(std::f64::consts::FRAC_PI_2 / 256.),
            )
            .unwrap();
    }
    assert!((state.voltage.into_base() - 2.).abs() < 1.1e-5);
    assert!((state.current.into_base() - 2.).abs() < 1e-9);
}
