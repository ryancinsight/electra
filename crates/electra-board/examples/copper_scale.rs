//! Benchmark independent columns, each with two circular pads and a connecting trace.
use electra_board::{
    board::NetId,
    geom::{Nm, Point},
    rules::contact::{Conductor, check},
};
use std::{error::Error, time::Instant};
fn main() -> Result<(), Box<dyn Error>> {
    let nets: u32 = std::env::args()
        .nth(1)
        .ok_or("usage: copper_scale NETS (1..=10000)")?
        .parse()?;
    if !(1..=10000).contains(&nets) {
        return Err("net count outside fixture range".into());
    }
    let mut objects = Vec::with_capacity(3 * nets as usize);
    for n in 0..nets {
        let x = Nm(i64::from(n) * 200_000);
        let a = Point::new(x, Nm(0));
        let b = Point::new(x, Nm(500_000));
        let pad = Conductor {
            net: NetId(n),
            axis: [a, a],
            width: Nm(80_000),
            terminal: true,
            layers: [0, 0],
            drill: None,
        };
        objects.extend([
            pad,
            Conductor {
                axis: [b, b],
                ..pad
            },
            Conductor {
                axis: [a, b],
                terminal: false,
                width: Nm(40_000),
                ..pad
            },
        ]);
    }
    let start = Instant::now();
    let mut issues = 0;
    check(
        &objects,
        Nm(40_000),
        Nm(0),
        Nm(0),
        &Default::default(),
        |_| issues += 1,
    )?;
    assert_eq!(issues, 0);
    println!("{} {}", objects.len(), start.elapsed().as_micros());
    Ok(())
}
