//! Driven series branch charge/energy balance and disk checkpoint replay.
use electra_circuit::{
    ElectricPotential, ElectricalImpedance,
    transient::{Capacitance, Inductance, RlcCheckpoint, RlcState, SeriesRlc, Time},
};
fn model() -> SeriesRlc {
    SeriesRlc::new(
        ElectricalImpedance::from_base(2.),
        Inductance::from_base(0.1),
        Capacitance::from_base(0.001),
    )
    .unwrap()
}
fn drive(k: u64) -> ElectricPotential {
    ElectricPotential::from_base(if k < 400 { 3. } else { 1. })
}

#[test]
fn conservation_and_disk_restart_at_drive_change() {
    let model = model();
    let dt = Time::from_base(0.0001);
    let mut state = RlcState {
        voltage: Default::default(),
        current: Default::default(),
    };
    let mut restart = None;
    let (mut charge, mut work, mut heat) = (0., 0., 0.);
    for k in 0..1000 {
        if k == 400 {
            let path =
                std::env::temp_dir().join(format!("electra-rlc-checkpoint-{}", std::process::id()));
            let mut file = std::fs::File::create_new(&path).unwrap();
            std::io::Write::write_all(&mut file, &model.checkpoint(state, dt, k).unwrap()).unwrap();
            drop(file);
            let record = std::fs::read(&path).unwrap();
            std::fs::remove_file(path).unwrap();
            restart = Some(RlcCheckpoint::decode(&record).unwrap());
        }
        let next = model.advance(state, drive(k), dt).unwrap();
        let current = 0.5 * (state.current.into_base() + next.current.into_base());
        charge += dt.into_base() * current;
        work += dt.into_base() * current * drive(k).into_base();
        heat += dt.into_base() * 2. * current * current;
        if let Some(saved) = &mut restart {
            assert_eq!(saved.next_step, k);
            saved.state = saved
                .model
                .advance(saved.state, drive(saved.next_step), saved.interval)
                .unwrap();
            saved.next_step += 1;
            assert_eq!(
                saved.state.voltage.into_base().to_bits(),
                next.voltage.into_base().to_bits()
            );
            assert_eq!(
                saved.state.current.into_base().to_bits(),
                next.current.into_base().to_bits()
            );
        }
        state = next;
    }
    let v = state.voltage.into_base();
    let i = state.current.into_base();
    let stored = 0.5 * (0.001 * v * v + 0.1 * i * i);
    assert!(charge.abs() > 1e-4 && work > 1e-3);
    let charge_error = (charge - 0.001 * v).abs() / charge.abs();
    let energy_error = (work - heat - stored).abs() / work;
    println!("charge error {charge_error:e}, energy error {energy_error:e}");
    assert!(charge_error < 0.002 && energy_error < 0.002);
}

#[test]
fn rejects_invalid_records() {
    let mut data = model()
        .checkpoint(
            RlcState {
                voltage: Default::default(),
                current: Default::default(),
            },
            Time::from_base(1.),
            u64::MAX,
        )
        .unwrap();
    assert_eq!(RlcCheckpoint::decode(&data).unwrap().next_step, u64::MAX);
    assert!(RlcCheckpoint::decode(&data[..63]).is_err());
    data[0] ^= 1;
    assert!(RlcCheckpoint::decode(&data).is_err());
    data[0] ^= 1;
    data[56..].copy_from_slice(&f64::NAN.to_le_bytes());
    assert!(RlcCheckpoint::decode(&data).is_err());
}
