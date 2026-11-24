//! Profile preparation and a streamed sweep of independent four-element cells.
//! Usage: mna_profile ELEMENTS reduced|mna
use electra_circuit::{MnaOptions, SourceValue, parse};
use std::{env, fmt::Write, time::Instant};

fn main() {
    let elements: usize = env::args().nth(1).expect("element count").parse().unwrap();
    let cells = elements / 4;
    assert!(cells > 0 && cells * 4 == elements);
    let mode = env::args().nth(2).expect("reduced or mna");
    let mut deck = String::new();
    // Each cell has a floating source. Only cell zero is swept, with an
    // independent common-mode supply and passive return in every cell.
    for i in 0..cells {
        writeln!(
            deck,
            "V{i} p{i} 0 9\nRu{i} p{i} a{i} 1k\nVs{i} a{i} b{i} 1\nRl{i} b{i} 0 2k"
        )
        .unwrap();
    }
    deck.push_str(".dc V0 0 10 0.1\n");
    let circuit = parse(&deck).unwrap();
    let start = Instant::now();
    let mut iterations = 0;
    let mut points = 0;
    let mut max_current = 0.0_f64;
    let mut max_voltage = 0.0_f64;
    let mut check = |excitation: SourceValue,
                     probes: &electra_circuit::DcResult<'_>,
                     kvl: f64,
                     steps: usize| {
        let SourceValue::Voltage(applied) = excitation else {
            unreachable!()
        };
        let q = applied.into_base();
        assert!((probes.volts[1].1.into_base() - q).abs() < 1e-10);
        let i = probes.amperes[0].1.into_base();
        let other = probes.amperes[(cells - 1) * 4].1.into_base();
        let kcl = probes.residual_amperes.into_base();
        assert!((i - (1. - q) / 3000.).abs() < 1e-12);
        if cells > 1 {
            assert!((other + 8. / 3000.).abs() < 1e-12);
        }
        max_current = max_current.max(kcl);
        max_voltage = max_voltage.max(kvl);
        iterations += steps;
        points += 1;
    };
    let prepared = match mode.as_str() {
        "mna" => {
            let mut run = circuit.dc_sweep_mna(MnaOptions::default()).unwrap();
            let prepared = start.elapsed();
            while let Some(p) = run.next_point().unwrap() {
                let r = p.result;
                check(
                    p.excitation,
                    &r.probes,
                    r.residual_volts.into_base(),
                    r.iterations,
                );
            }
            prepared
        }
        "reduced" => {
            let mut run = circuit.dc_sweep(Default::default()).unwrap();
            let prepared = start.elapsed();
            while let Some(p) = run.next_point().unwrap() {
                let r = p.result;
                check(p.excitation, r, 0., r.sweeps);
            }
            prepared
        }
        _ => panic!("expected reduced or mna"),
    };
    let total = start.elapsed();
    assert_eq!(points, 101);
    println!(
        "{:.6} {:.6} {iterations} {max_current:.4e} {max_voltage:.4e}",
        prepared.as_secs_f64() * 1000.,
        total.as_secs_f64() * 1000.
    );
}
