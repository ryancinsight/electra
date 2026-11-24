//! Benchmarks an entire circuit sweep including preparation and named probe projection.
use electra_circuit::parse;
use std::{env, fmt::Write, hint::black_box, time::Instant};

fn main() {
    let size: usize = env::args().nth(1).expect("cell count").parse().unwrap();
    assert!(size > 0);
    let floating = match env::args().nth(2).as_deref() {
        Some("floating") => true,
        Some("grounded") => false,
        _ => panic!("expected grounded or floating"),
    };
    let mut deck = String::from("Vin rail 0 1\n.dc Vin 0 1 0.01\n");
    for i in 0..size {
        writeln!(deck, "Ra{i} rail a{i} 1k").unwrap();
        if floating {
            writeln!(deck, "Vcell{i} a{i} b{i} 1\nRb{i} b{i} 0 2k").unwrap();
        } else {
            writeln!(deck, "Rb{i} a{i} 0 2k").unwrap();
        }
    }
    let circuit = parse(&deck).unwrap();
    let start = Instant::now();
    let mut run = circuit.dc_sweep(Default::default()).unwrap();
    let mut sum = 0.0;
    while let Some(point) = run.next_point().unwrap() {
        sum += black_box(point.result.amperes[0].1.into_base());
    }
    let expected =
        (if floating { 50.5 } else { -50.5 }) * f64::from(u32::try_from(size).unwrap()) / 3000.;
    assert!((sum - expected).abs() < 1e-9 * expected.abs());
    println!("{:.3} {sum:.12}", 1000.0 * start.elapsed().as_secs_f64());
}
