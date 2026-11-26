//! Time passive R/C/L stamps into a caller-owned tridiagonal matrix and RHS.
use electra_ac::{
    Capacitance, Complex64, ElectricPotential, ElectricalImpedance, Frequency, Inductance,
    nodal::{PassiveBranch, PassiveElement},
};
use std::{error::Error, hint::black_box, time::Instant};

fn main() -> Result<(), Box<dyn Error>> {
    let nodes: usize = std::env::args()
        .nth(1)
        .ok_or("expected node count")?
        .parse()?;
    if nodes < 2 {
        return Err("at least two nodes required".into());
    }
    let elements = [
        PassiveElement::resistor(ElectricalImpedance::from_base(1000.))?,
        PassiveElement::capacitor(Capacitance::from_base(1e-6))?,
        PassiveElement::inductor(Inductance::from_base(1e-3))?,
    ];
    let edges: Vec<_> = (0..nodes - 1)
        .map(|i| PassiveBranch::new(i, i + 1, elements[i % 3]))
        .collect();
    let mut boundary = vec![None; nodes];
    boundary[0] = Some(ElectricPotential::from_base(Complex64::from(1.)));
    let mut bands = vec![[Complex64::default(); 3]; nodes];
    let mut rhs = vec![Complex64::default(); nodes];
    let start = Instant::now();
    for hz in 1..=101 {
        bands.fill([Complex64::default(); 3]);
        rhs.fill(Complex64::default());
        for edge in &edges {
            edge.stamp(
                Frequency::from_base(f64::from(hz)),
                &boundary,
                |i, j, y| bands[i][j + 1 - i] += y.into_base(),
                |i, b| rhs[i] += b.into_base(),
            )?;
        }
        black_box((&bands, &rhs));
    }
    println!(
        "nodes={nodes} stamps={} elapsed_ns={} buffer_bytes={}",
        101 * edges.len(),
        start.elapsed().as_nanos(),
        size_of_val(edges.as_slice())
            + size_of_val(boundary.as_slice())
            + size_of_val(bands.as_slice())
            + size_of_val(rhs.as_slice())
    );
    Ok(())
}
