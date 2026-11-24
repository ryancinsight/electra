//! Measure 101 DC right-hand sides on a grounded chain with one shunt per node.
use aequitas::systems::si::quantities::{ElectricConductance, ElectricCurrent, ElectricPotential};
use electra_network::{Branch, PreparedNetwork, solve};
use std::{env, hint::black_box, time::Instant};

fn main() {
    let size: usize = env::args().nth(1).expect("node count").parse().unwrap();
    let fixed: Vec<_> = std::iter::once(Some(ElectricPotential::default()))
        .chain(std::iter::repeat_n(None, size))
        .collect();
    let mut loads = vec![ElectricCurrent::default(); fixed.len()];
    let mut edges = Vec::new();
    for node in 1..=size {
        edges.push(Branch {
            from: node,
            to: 0,
            conductance: ElectricConductance::from_base(1.0),
        });
        if node > 1 {
            edges.push(Branch {
                from: node - 1,
                to: node,
                conductance: ElectricConductance::from_base(0.25),
            });
        }
    }
    let mode = env::args().nth(2).expect("prepared or rebuild");
    let start = Instant::now();
    let mut checksum = 0.0;
    let mut prepared = match mode.as_str() {
        "prepared" => Some(PreparedNetwork::new(&fixed, &edges).unwrap()),
        "rebuild" => None,
        _ => panic!("expected prepared or rebuild"),
    };
    for index in 0..=100 {
        loads[1] = ElectricCurrent::from_base(f64::from(index) / 100.0);
        let potential = match &mut prepared {
            Some(network) => {
                network
                    .solve(&fixed, &loads, Default::default())
                    .unwrap()
                    .potentials[1]
            }
            None => {
                solve(&fixed, &loads, &edges, Default::default())
                    .unwrap()
                    .potentials[1]
            }
        };
        checksum += black_box(potential.into_base());
    }
    println!(
        "{size} {:.3} {checksum:.12}",
        start.elapsed().as_secs_f64() * 1000.0
    );
}
