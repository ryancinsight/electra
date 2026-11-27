//! Compare synchronized rectified drive and sampled ADC transitions to exact LC propagation.
use electra_circuit::{
    ElectricCurrent, ElectricPotential, ElectricalImpedance,
    transient::{
        AdaptiveOptions, AdcEdge, Capacitance, Inductance, RlcCheckpoint, RlcState, SeriesRlc, Time,
    },
};
use std::time::Instant;
#[derive(Debug, PartialEq)]
struct Trace {
    edges: Vec<(u64, AdcEdge)>,
    samples: Vec<[u64; 2]>,
    accepted: usize,
    rejected: usize,
}

fn run(restart: bool) -> Trace {
    let mut sim = RlcCheckpoint {
        model: SeriesRlc::new(
            ElectricalImpedance::default(),
            Inductance::from_base(1.),
            Capacitance::from_base(1.),
        )
        .unwrap(),
        state: RlcState {
            voltage: Default::default(),
            current: Default::default(),
        },
        interval: Time::from_base(0.1),
        next_step: 0,
    };
    let controls = AdaptiveOptions {
        maximum_step: sim.interval,
        voltage_error: ElectricPotential::from_base(1e-7),
        current_error: ElectricCurrent::from_base(1e-7),
    };
    let initial = sim.state;
    assert!(
        sim.advance_clock(
            ElectricPotential::from_base(f64::NAN),
            ElectricPotential::default(),
            &controls
        )
        .is_err()
    );
    assert_eq!(sim.next_step, 0);
    assert_eq!(sim.state, initial);
    let threshold = 0.57;
    let (s, c) = 0.1_f64.sin_cos();
    let (mut v, mut i) = (0., 0.);
    let (mut accepted, mut rejected) = (0, 0);
    let mut edges = Vec::new();
    let mut samples = Vec::new();
    let mut resumed = false;
    let mut max_v = 0.0_f64;
    let mut max_i = 0.0_f64;
    let start = Instant::now();
    for k in 0..400 {
        assert_eq!(sim.next_step, k);
        // Ideal half-wave rectifier on a clocked bipolar square-wave source.
        let raw: f64 = if sim.next_step % 16 < 8 { 1. } else { -1. };
        let drive = raw.max(0.);
        let above = v >= threshold;
        (v, i) = (drive + (v - drive) * c + i * s, i * c - (v - drive) * s);
        let report = sim
            .advance_clock(
                ElectricPotential::from_base(drive),
                ElectricPotential::from_base(threshold),
                &controls,
            )
            .unwrap();
        let reference = match (above, v >= threshold) {
            (false, true) => Some(AdcEdge::Rising),
            (true, false) => Some(AdcEdge::Falling),
            _ => None,
        };
        assert_eq!(report.edge, reference, "clock {}", sim.next_step);
        if let Some(edge) = report.edge {
            edges.push((sim.next_step, edge));
        }
        if restart && (sim.next_step == 8 || report.edge.is_some() && !resumed) {
            let record = sim
                .model
                .checkpoint(sim.state, sim.interval, sim.next_step)
                .unwrap();
            sim = RlcCheckpoint::decode(&record).unwrap();
            resumed |= report.edge.is_some();
        }
        let sample = [sim.state.voltage.into_base(), sim.state.current.into_base()];
        max_v = max_v.max((sample[0] - v).abs());
        max_i = max_i.max((sample[1] - i).abs());
        samples.push(sample.map(f64::to_bits));
        accepted += report.accepted;
        rejected += report.rejected;
    }
    assert!(!edges.is_empty() && resumed == restart);
    println!(
        "restart={restart} edges={} accepted={accepted} rejected={rejected} max_v={max_v:e} max_i={max_i:e} elapsed={:?}",
        edges.len(),
        start.elapsed()
    );
    assert!(max_v < 0.001 && max_i < 0.001);
    Trace {
        edges,
        samples,
        accepted,
        rejected,
    }
}
#[test]
fn restart_after_adc_edge_preserves_waveform_and_event_sequence() {
    assert_eq!(run(false), run(true));
}
