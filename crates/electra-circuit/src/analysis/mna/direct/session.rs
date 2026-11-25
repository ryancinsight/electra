use super::{DirectFactor, Matrix};
use crate::Circuit;
use crate::analysis::excitation::Excitation;
use crate::analysis::mna::{Balance, Drive, MnaError, MnaOptions, MnaPoint, System};
use athena_core::LinearOperator;

pub(super) struct Session<'a, F> {
    circuit: &'a Circuit,
    system: System,
    factor: F,
    rhs: Vec<f64>,
    solution: Vec<f64>,
    drive: Drive,
    balance: Vec<Balance>,
    limits: MnaOptions,
    pub point: MnaPoint<'a>,
}
impl<'a, F: DirectFactor> Session<'a, F> {
    pub fn new(c: &'a Circuit, limits: MnaOptions, swept: Option<usize>) -> Result<Self, MnaError> {
        limits.validate()?;
        let system = System::new(c, F::MAX_ORDER)?;
        let factor = F::factor(Matrix(&system))?;
        let mut rhs = vec![0.; system.dimension()];
        let drive = system.rhs(c, swept, &mut rhs)?;
        Ok(Self {
            circuit: c,
            factor,
            drive,
            limits,
            solution: vec![0.; rhs.len()],
            rhs,
            balance: vec![Balance::default(); c.names.len()],
            point: MnaPoint::new(c),
            system,
        })
    }
    pub fn run(&mut self, excitation: Excitation) -> Result<(), MnaError> {
        if let Some((_, value)) = excitation {
            self.drive.update(value, &mut self.rhs)?;
        }
        self.factor
            .solve_into(&self.rhs[1..], &mut self.solution[1..])?;
        self.point.project(
            self.circuit,
            &self.system,
            &self.solution,
            excitation,
            self.limits,
            &mut self.balance,
        )
    }
}
