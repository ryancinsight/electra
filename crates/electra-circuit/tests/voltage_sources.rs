//! Acceptance fixtures for independent voltage constraints and original-node conservation.
#[path = "voltage_sources/constraints.rs"]
mod constraints;
#[path = "voltage_sources/currents.rs"]
mod currents;
#[path = "voltage_sources/reduction.rs"]
mod reduction;

use aequitas::Quantity;
fn near<D>(actual: Quantity<f64, D>, expected: f64) {
    let tolerance = 1e-12 + 1e-9 * expected.abs();
    assert!(
        (actual.into_base() - expected).abs() <= tolerance,
        "{} vs {expected}",
        actual.into_base()
    );
}
