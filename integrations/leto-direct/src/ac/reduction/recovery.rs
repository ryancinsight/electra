//! Recover eliminated voltages in reverse elimination order.
use super::quotient;
use electra_ac::{Complex64, ElectricPotential, PhasorSum};
use std::error::Error;
pub(super) struct Step<E> {
    pub(super) slot: usize,
    pub(super) diagonal: Complex64,
    pub(super) rhs: Complex64,
    pub(super) edges: E,
}

pub(super) enum Recovery {
    Scalar(Step<[Option<(usize, Complex64)>; 2]>),
    Constraint(Step<Vec<(usize, Complex64)>>),
}

fn recover(
    slot: usize,
    diagonal: Complex64,
    value: Complex64,
    edges: impl Iterator<Item = (usize, Complex64)>,
    free: &[usize],
    volts: &mut [ElectricPotential<Complex64>],
) -> Result<(), Box<dyn Error>> {
    let mut rhs = PhasorSum::default();
    rhs.add(value);
    for (j, y) in edges {
        rhs.add(-y * volts[free[j]].into_base());
    }
    volts[free[slot]] = ElectricPotential::from_base(quotient(rhs.total(), diagonal)?);
    Ok(())
}

impl Recovery {
    pub(super) fn apply(
        self,
        free: &[usize],
        volts: &mut [ElectricPotential<Complex64>],
    ) -> Result<(), Box<dyn Error>> {
        match self {
            Self::Scalar(s) => recover(
                s.slot,
                s.diagonal,
                s.rhs,
                s.edges.into_iter().flatten(),
                free,
                volts,
            ),
            Self::Constraint(s) => {
                recover(s.slot, s.diagonal, s.rhs, s.edges.into_iter(), free, volts)
            }
        }
    }
}
