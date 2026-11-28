//! Read round pad, straight track and via records and report gaps in integer PCB units.
use electra_board::{
    board::NetId,
    geom::{Nm, Point},
    rules::contact::{Conductor, Finding, check},
};
use std::{
    collections::BTreeSet,
    error::Error,
    io::{self, BufRead},
};
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [gap, drill, ring] = args.as_slice() else {
        return Err("usage: electra-copper CLEARANCE_NM DRILL_NM RING_NM".into());
    };
    let mut items = Vec::new();
    let mut deferred = BTreeSet::new();
    for line in io::stdin().lock().lines() {
        let line = line?;
        let mut f = line.split_whitespace();
        let kind = f.next().ok_or("missing kind")?;
        let mut next = || f.next().ok_or("missing field");
        let net = NetId(next()?.parse()?);
        if kind == "defer" {
            deferred.insert(net);
        } else {
            if !matches!(kind, "pad" | "via" | "track") {
                return Err("unknown object".into());
            }
            let a = Point::new(Nm(next()?.parse()?), Nm(next()?.parse()?));
            let b = Point::new(Nm(next()?.parse()?), Nm(next()?.parse()?));
            items.push(Conductor {
                net,
                axis: [a, b],
                width: Nm(next()?.parse()?),
                layers: [next()?.parse()?, next()?.parse()?],
                terminal: kind == "pad",
                drill: if kind == "via" {
                    Some(Nm(next()?.parse()?))
                } else {
                    None
                },
            });
        }
        if f.next().is_some() {
            return Err("trailing field".into());
        }
    }
    check(
        &items,
        Nm(gap.parse()?),
        Nm(drill.parse()?),
        Nm(ring.parse()?),
        &deferred,
        |finding| match finding {
            Finding::Short(a, b) => println!("short {a} {b}"),
            Finding::Clearance(a, b) => println!("clearance {a} {b}"),
            Finding::Open(n) => println!("open {}", n.0),
            Finding::Drill(i) => println!("drill {i}"),
            Finding::Ring(i) => println!("annular_ring {i}"),
        },
    )?;
    Ok(())
}
