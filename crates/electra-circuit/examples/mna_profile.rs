//! Profile preparation and a streamed sweep of repeated cells.
//! Usage: mna_profile ELEMENTS reduced|mna|controlled
use electra_circuit::{MnaOptions, SourceValue, parse};
use std::{env, fmt::Write, time::Instant};

fn main() {
    let elements: usize = env::args().nth(1).expect("element count").parse().unwrap();
    let mode = env::args().nth(2).expect("reduced, mna or controlled");
    let controlled = mode == "controlled";
    let per_cell = if controlled { 10 } else { 4 };
    let cells = elements / per_cell;
    assert!(cells > 0 && cells * per_cell == elements);
    let mut deck = String::new();
    if controlled {
        deck.push_str(
            ".subckt cell
Vin in 0 9
Rin in 0 1k
Eamp e 0 in e 2
Re e 0 1k
Gout g 0 in 0 2m
Rg g 0 1k
Fcopy f 0 Vin 2
Rf f 0 1k
Hout h 0 Vin -3k
Rh h 0 1k
.ends
",
        );
        for i in 0..cells {
            writeln!(deck, "X{i} cell").unwrap();
        }
        deck.push_str(
            ".dc X0/Vin 0 10 0.1
",
        );
    } else {
        for i in 0..cells {
            writeln!(
                deck,
                "V{i} p{i} 0 9\nRu{i} p{i} a{i} 1k\nVs{i} a{i} b{i} 1\nRl{i} b{i} 0 2k"
            )
            .unwrap();
        }
        deck.push_str(".dc V0 0 10 0.1\n");
    }
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
        let other = probes.amperes[(cells - 1) * per_cell].1.into_base();
        let kcl = probes.residual_amperes.into_base();
        if controlled {
            for (n, factor) in [(2, 2. / 3.), (3, -2.), (4, 2.), (5, 3.)] {
                assert!((probes.volts[n].1.into_base() - q * factor).abs() < 1e-9);
            }
            assert!((i + q / 1000.).abs() < 1e-12);
            if cells > 1 {
                assert!((other + 0.009).abs() < 1e-12);
            }
        } else {
            assert!((i - (1. - q) / 3000.).abs() < 1e-12);
            if cells > 1 {
                assert!((other + 8. / 3000.).abs() < 1e-12);
            }
        }
        max_current = max_current.max(kcl);
        max_voltage = max_voltage.max(kvl);
        iterations += steps;
        points += 1;
    };
    let prepared = match mode.as_str() {
        "mna" | "controlled" => {
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
        _ => panic!("expected reduced, mna or controlled"),
    };
    let total = start.elapsed();
    assert_eq!(points, 101);
    println!(
        "{:.6} {:.6} {iterations} {max_current:.4e} {max_voltage:.4e}",
        prepared.as_secs_f64() * 1000.,
        total.as_secs_f64() * 1000.
    );
}
