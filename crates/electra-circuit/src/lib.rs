//! Electrical DC analysis for resistor networks and independent sources.
//! Use [`parse`] to read a deck, [`Circuit::operating_point`] for nominal bias and
//! [`Circuit::dc_sweep`] to stream a DC transfer curve.
mod analysis;
mod netlist;
use aequitas::systems::si::dimensions;
pub use aequitas::systems::si::quantities::{
    Dimensionless, ElectricConductance, ElectricCurrent, ElectricPotential, ElectricalImpedance,
};
pub use analysis::{DcResult, DcSample, DcSweep, LinearSweep, SourceValue, SweepError, SweepRange};
use electra_network::Branch;
/// Electrical convergence limits measured in amperes and dimensionless relative error.
/// ```compile_fail
/// use electra_circuit::{ElectricPotential, SolveOptions};
/// let _ = SolveOptions { absolute: ElectricPotential::from_base(1e-9), ..Default::default() };
/// ```
pub type SolveOptions = electra_network::SolveOptions<dimensions::ElectricCurrent>;
/// Electrical solver failure with a current-valued residual on nonconvergence.
pub type SolveError = electra_network::Error<dimensions::ElectricCurrent>;
pub use netlist::{ParseError, parse};

#[derive(Debug)]
enum Element {
    Resistor { branch: usize },
    Current { amperes: ElectricCurrent },
    Voltage { prescribed: usize },
}
#[derive(Debug)]
struct Part {
    name: String,
    positive: usize,
    negative: usize,
    element: Element,
}

fn prescribed_voltage(
    positive: usize,
    negative: usize,
    value: ElectricPotential,
) -> (usize, ElectricPotential) {
    if negative == 0 {
        (positive, value)
    } else {
        (negative, -value)
    }
}

/// A checked, named circuit. Node zero is the reference.
#[derive(Debug)]
pub struct Circuit {
    names: Vec<String>,
    fixed_volts: Vec<Option<ElectricPotential>>,
    sweep: Option<analysis::sweep::SweepPlan>,
    branches: Vec<Branch<dimensions::ElectricConductance>>,
    parts: Vec<Part>,
}
