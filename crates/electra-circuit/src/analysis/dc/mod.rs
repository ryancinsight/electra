//! Prepare electrical excitations once, then solve and project into original circuit coordinates.
mod readout;
mod supernode;

use super::excitation::{CurrentSum, Excitation, injections, voltage};
use crate::{
    Circuit, DcResult, ElectricCurrent, ElectricPotential, Element, SolveError, SolveOptions,
    SourceValue, constraints::Constraints,
};
use aequitas::systems::si::dimensions;
use electra_network::{OperatingPoint, PreparedNetwork};
use supernode::Supernodes;

type ElectricalNetwork<'a> = PreparedNetwork<
    'a,
    dimensions::ElectricPotential,
    dimensions::ElectricConductance,
    dimensions::ElectricCurrent,
>;
type Point = OperatingPoint<dimensions::ElectricPotential, dimensions::ElectricCurrent>;
struct CurrentPorts {
    a: usize,
    b: usize,
    base_a: CurrentSum,
    base_b: CurrentSum,
}

pub(super) struct Session<'a> {
    circuit: &'a Circuit,
    network: ElectricalNetwork<'a>,
    fixed: Vec<Option<ElectricPotential>>,
    rhs: Vec<ElectricCurrent>,
    ports: Option<CurrentPorts>,
    coupled: Option<Supernodes<'a>>,
    pub output: DcResult<'a>,
}

impl<'a> Session<'a> {
    pub fn new(c: &'a Circuit, swept: Option<usize>) -> Result<Self, SolveError> {
        let (fixed, branches, rhs, coupled) = match &c.constraints {
            Constraints::Grounded(fixed) => (
                fixed.clone(),
                &c.branches,
                injections(c, fixed.len(), swept, |n| n),
                None,
            ),
            Constraints::Coupled(forest) => {
                let mut fixed = vec![None; forest.roots.len()];
                fixed[0] = Some(ElectricPotential::default());
                let rhs = injections(c, fixed.len(), swept, |n| forest.group[n]);
                let (solver, rhs) = Supernodes::prepare(c, forest, swept, rhs)?;
                (fixed, &forest.branches, rhs, Some(solver))
            }
        };
        let group = |node: usize| coupled.as_ref().map_or(node, |s| s.forest.group[node]);
        let ports = swept.and_then(|index| {
            let p = &c.parts[index];
            let (a, b) = (group(p.positive), group(p.negative));
            (matches!(p.element, Element::Current { .. }) && a != b).then(|| CurrentPorts {
                a,
                b,
                base_a: rhs[a],
                base_b: rhs[b],
            })
        });
        let rhs = rhs.into_iter().map(CurrentSum::value).collect();
        let network =
            ElectricalNetwork::new(&fixed, branches).map_err(|err| match (&coupled, err) {
                (Some(s), SolveError::Floating(group)) => {
                    SolveError::Floating(s.forest.roots[group])
                }
                (_, other) => other,
            })?;
        Ok(Self {
            circuit: c,
            fixed,
            rhs,
            network,
            coupled,
            ports,
            output: DcResult::new(c),
        })
    }

    pub fn run(&mut self, excitation: Excitation, options: SolveOptions) -> Result<(), SolveError> {
        if let Some((index, applied)) = excitation {
            let p = &self.circuit.parts[index];
            match applied {
                SourceValue::Voltage(v) => {
                    if let Some(s) = &mut self.coupled {
                        s.set_voltage(self.circuit, excitation, &mut self.rhs)?;
                    } else {
                        let node = p.positive.max(p.negative);
                        self.fixed[node] = Some(p.step_to(node, v));
                    }
                }
                SourceValue::Current(i) => {
                    if let Some(ports) = &self.ports {
                        self.rhs[ports.a] = ports.base_a.with(-i);
                        self.rhs[ports.b] = ports.base_b.with(i);
                    }
                }
            }
        }
        if let Some(s) = &mut self.coupled {
            self.network
                .solve_checked(&self.fixed, &self.rhs, options, |point| {
                    s.project(self.circuit, point, excitation, options, &mut self.output)
                })?;
        } else {
            let point = self.network.solve(&self.fixed, &self.rhs, options)?;
            self.output.grounded(self.circuit, point, excitation);
        }
        Ok(())
    }
}
