//! Import the normalized physical network exported by KiCad.
use kwavers_driver::{
    board::{Board, LayerId, NetClassKind},
    geom::{GridSpec, Nm, Point},
    place::{Component, FootprintDef, PadDef, Placement, Role},
};
use std::collections::BTreeMap;
use std::error::Error;
pub(crate) struct Input {
    pub board: Board,
    pub parts: Vec<Component>,
    pub library: Vec<FootprintDef>,
}
pub(crate) fn load(text: &str) -> Result<Input, Box<dyn Error>> {
    let mut lines = text.lines();
    let header: Vec<_> = lines
        .next()
        .ok_or("missing board")?
        .split_whitespace()
        .collect();
    let ["board", w, h, layers] = header[..] else {
        return Err("invalid header".into());
    };
    let spec = GridSpec::cover(Nm(w.parse()?), Nm(h.parse()?), Nm(500_000), layers.parse()?)?;
    let mut board = Board::new(spec);
    let mut parts: Vec<Component> = Vec::new();
    let mut library: Vec<FootprintDef> = Vec::new();
    let mut nets = BTreeMap::new();
    for line in lines {
        let words: Vec<_> = line.split_whitespace().collect();
        match words[..] {
            ["part", reference, x, y, w, h, lock] => {
                let role = if lock == "1" {
                    Role::Connector
                } else if reference.starts_with('U') {
                    Role::ActiveIc
                } else {
                    Role::Passive
                };
                parts.push(Component {
                    fp: library.len(),
                    refdes: reference.into(),
                    locked: lock == "1",
                    placement: Placement {
                        pos: Point::new(Nm(x.parse()?), Nm(y.parse()?)),
                        ..Default::default()
                    },
                    ..Default::default()
                });
                library.push(FootprintDef::new(
                    reference,
                    (Nm(w.parse()?), Nm(h.parse()?)),
                    role,
                    vec![],
                ));
            }
            ["pad", x, y, w, h, net, layers] => {
                let id: u32 = net.parse()?;
                let net = (id > 0).then(|| {
                    *nets
                        .entry(id)
                        .or_insert_with(|| board.add_net(format!("N{id}"), NetClassKind::Signal))
                });
                parts.last_mut().ok_or("pad without part")?.nets.push(net);
                let layers = layers
                    .split(',')
                    .map(|s| s.parse().map(LayerId))
                    .collect::<Result<_, _>>()?;
                library
                    .last_mut()
                    .ok_or("pad without part")?
                    .pads
                    .push(PadDef {
                        offset: Point::new(Nm(x.parse()?), Nm(y.parse()?)),
                        size: (Nm(w.parse()?), Nm(h.parse()?)),
                        layers,
                        power_pin: false,
                    });
            }
            _ => return Err("malformed input record".into()),
        }
    }
    Ok(Input {
        board,
        parts,
        library,
    })
}
