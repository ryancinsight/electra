//! Electrical DC analysis for resistor networks with independent and linear controlled sources.
//! Use [`parse`] to read a deck, [`Circuit::operating_point`] for nominal bias and
//! [`Circuit::dc_sweep`] to stream a DC transfer curve. [`Circuit::operating_point_mna`]
//! and [`Circuit::dc_sweep_mna`] select the full nodal and branch-current system.
mod analysis;
mod constraints;
mod device;
mod netlist;
pub mod semiconductor;
pub mod tolerance;
pub mod transient;
use aequitas::systems::si::dimensions;
pub use aequitas::systems::si::quantities::{
    Dimensionless, ElectricConductance, ElectricCurrent, ElectricPotential, ElectricalImpedance,
};
pub use analysis::mna::direct;
pub use analysis::mna::{MnaError, MnaOptions, MnaPoint, MnaSample, MnaSweep};
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
pub use netlist::ac::{AcDeck, AcSystem, parse_ac};
pub use netlist::{FileError, ParseError, parse, parse_file};

#[derive(Debug)]
enum Element {
    Controlled(usize),
    Resistor { branch: usize },
    Current { amperes: ElectricCurrent },
    Voltage { volts: ElectricPotential },
}
#[derive(Debug)]
struct Part {
    name: String,
    positive: usize,
    negative: usize,
    element: Element,
}

type Resistor = Branch<dimensions::ElectricConductance>;

/// A checked, named circuit. Node zero is the reference.
#[derive(Debug)]
pub struct Circuit {
    names: Vec<String>,
    constraints: constraints::Constraints,
    sweep: Option<analysis::sweep::SweepPlan>,
    branches: Vec<Resistor>,
    parts: Vec<Part>,
    controlled: Vec<device::Controlled>,
}

impl Circuit {
    /// Whether DC analysis requires the general modified nodal solver.
    pub fn requires_mna(&self) -> bool {
        !self.controlled.is_empty()
    }

    fn voltage_output(&self, part: &Part) -> bool {
        match part.element {
            Element::Voltage { .. } => true,
            Element::Controlled(i) => self.controlled[i].voltage_output(),
            _ => false,
        }
    }
}
