//! Compare placement policies on a fixed physical network and driver router.
mod input;
mod research;
use kwavers_driver::{
    board::{LayerId, NetClassKind},
    cost::RoutingCost,
    geom::{Nm, Point},
    pipeline::place_to_board,
    place::{AnnealParams, PlaceConfig, anneal},
    route::{Grid, PathFinderParams, Router},
    rules::DesignRules,
};
use std::error::Error;
struct Uniform;
impl RoutingCost for Uniform {
    fn node_base(&self, _: Point, _: LayerId, _: NetClassKind) -> f64 {
        1.
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [file, policy, seed, output] = args.as_slice() else {
        return Err(
            "usage: electra-place-study INPUT driver|research SEED OUTPUT.kicad_pcb".into(),
        );
    };
    let mut data = input::load(&std::fs::read_to_string(file)?)?;
    let seed = seed.parse()?;
    let spec = data.board.spec;
    let cfg = PlaceConfig {
        board: (
            spec.pitch * ((spec.nx - 1) as i64),
            spec.pitch * ((spec.ny - 1) as i64),
        ),
        margin: Nm(500_000),
        ..Default::default()
    };
    let movable = data
        .parts
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.locked)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    match policy.as_str() {
        "driver" => {
            anneal(
                &mut data.parts,
                &data.library,
                &cfg,
                &movable,
                &AnnealParams {
                    steps: 400,
                    seed,
                    ..Default::default()
                },
                None,
            );
        }
        "research" => research::place(&mut data, &cfg, &movable, seed),
        _ => return Err("unknown placement policy".into()),
    }
    let rules = DesignRules::default();
    let pins = place_to_board(&mut data.board, &data.parts, &data.library, &rules);
    let mut router = Router::new(Grid::new(spec), Uniform).with_params(PathFinderParams {
        max_iter: 3,
        ..Default::default()
    });
    let result = router.route_with_obstacles(&pins.terminals, &pins.obstacles);
    router.apply_to_board(&mut data.board, &pins.terminals, &result, &rules);
    let length: f64 = data
        .board
        .tracks
        .iter()
        .map(|t| t.start.euclid(t.end) * 1e-6)
        .sum();
    let lvs = kwavers_driver::verify::lvs(&data.board);
    let connected = result.routes.iter().filter(|r| r.connected).count();
    std::fs::write(
        output,
        kwavers_driver::io::write_kicad_pcb(&data.board, &data.parts, &data.library, &rules),
    )?;
    println!(
        "{connected},{},{},{},{},{},{length:.5}",
        result.routes.len(),
        result.overused_nodes,
        lvs.opens.len(),
        lvs.shorts.len(),
        data.board.vias.len()
    );
    Ok(())
}
