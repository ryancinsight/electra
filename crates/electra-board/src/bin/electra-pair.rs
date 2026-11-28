//! Evaluate a PCB pair from a stream of native KiCad segment coordinates in nanometres.
use electra_board::{
    board::NetId,
    geom::{Nm, Point},
    rules::{ConstraintId, PairRule},
};
use std::{
    error::Error,
    io::{self, BufRead},
};
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [id, p, n, length, skew] = args.as_slice() else {
        return Err("usage: electra-pair ID P_NET N_NET MAX_LENGTH_NM MAX_SKEW_NM".into());
    };
    let rule = PairRule {
        id: ConstraintId(id.parse()?),
        nets: [NetId(p.parse()?), NetId(n.parse()?)],
        length: Nm(length.parse()?),
        skew: Nm(skew.parse()?),
    };
    let mut scan = rule.sweep()?;
    let mut row = String::new();
    let mut source = io::stdin().lock();
    while source.read_line(&mut row)? != 0 {
        let mut fields = row.split_whitespace();
        let mut next = || fields.next().ok_or("expected net and four coordinates");
        let net = NetId(next()?.parse()?);
        let start = Point::new(Nm(next()?.parse()?), Nm(next()?.parse()?));
        let end = Point::new(Nm(next()?.parse()?), Nm(next()?.parse()?));
        if fields.next().is_some() {
            return Err("trailing segment data".into());
        }
        scan.segment(net, start, end);
        row.clear();
    }
    let m = scan.finish()?;
    println!(
        "{} {} {} {} {}",
        rule.id.0, m.lengths[0], m.lengths[1], m.length, m.skew
    );
    Ok(())
}
