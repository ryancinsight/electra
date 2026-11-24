//! Measure deck construction separately from one operating point on the same dividers.
use electra_circuit::parse;
use std::{env, fmt::Write, hint::black_box, time::Instant};

fn main() {
    let n: u32 = env::args().nth(1).expect("cell count").parse().unwrap();
    assert!(n > 0);
    let mode = env::args().nth(2).expect("flat or hierarchical");
    let mut deck = String::from("Vin rail 0 9\n");
    if mode == "hierarchical" {
        deck.push_str(
            ".subckt cell supply ret params: r=1kOhm\nRup supply mid {r}\nRlo mid ret 2k\n.ends\n",
        );
    } else {
        assert_eq!(mode, "flat");
    }
    for i in 0..n {
        if mode == "hierarchical" {
            let arg = if i % 2 == 0 { "" } else { " r=4k" };
            writeln!(deck, "X{i} rail 0 cell{arg}").unwrap();
        } else {
            let r = if i % 2 == 0 { "1k" } else { "4k" };
            writeln!(deck, "Rup{i} rail mid{i} {r}\nRlo{i} mid{i} 0 2k").unwrap();
        }
    }
    let start = Instant::now();
    let circuit = parse(black_box(&deck)).unwrap();
    let parsed = start.elapsed();
    let solve = Instant::now();
    let point = circuit.operating_point(Default::default()).unwrap();
    let solved = solve.elapsed();
    let current = point.amperes[0].1.into_base();
    let expected = -0.003 * f64::from(n.div_ceil(2)) - 0.0015 * f64::from(n / 2);
    assert!((current - expected).abs() < 1e-10 * expected.abs());
    println!(
        "{:.3} {:.3} {current:.12}",
        1000. * parsed.as_secs_f64(),
        1000. * solved.as_secs_f64()
    );
}
