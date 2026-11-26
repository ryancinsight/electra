//! Two-terminal passive AC netlist elements for reduced nodal assembly.
//! Nodes use the caller's numbering. A fixed phasor eliminates its node from
//! the free equations. Each callback adds a contribution to the supplied row.
//!
//! At one radian per second, a one-ohm resistor feeding a one-farad
//! capacitor gives an output of (1 − j)/2 volts for a one-volt drive.
//!
//! ```
//! use electra_ac::{Capacitance, Complex64, ElectricPotential, ElectricalImpedance, Frequency};
//! use electra_ac::nodal::{PassiveBranch, PassiveElement};
//!
//! let drive = ElectricPotential::from_base(Complex64::from(1.));
//! let fixed = [Some(drive), None, Some(ElectricPotential::default())];
//! let branches = [
//!     PassiveBranch::new(0, 1, PassiveElement::resistor(ElectricalImpedance::from_base(1.))?),
//!     PassiveBranch::new(1, 2, PassiveElement::capacitor(Capacitance::from_base(1.))?),
//! ];
//! let mut diagonal = electra_ac::ElectricConductance::default();
//! let mut rhs = electra_ac::ElectricCurrent::default();
//! for edge in branches {
//!     edge.stamp(Frequency::from_base(core::f64::consts::TAU.recip()), &fixed,
//!         |_, _, y| diagonal += y, |_, b| rhs += b)?;
//! }
//! let output: ElectricPotential<Complex64> = rhs / diagonal;
//! assert_eq!(output.into_base(), Complex64::new(0.5, -0.5));
//! # Ok::<(), electra_ac::nodal::StampError>(())
//! ```
mod element;
mod error;

use crate::{Complex64, ElectricConductance, ElectricCurrent, ElectricPotential, Frequency};
use electra_network::Branch;
pub use element::PassiveElement;
pub use error::StampError;

/// A validated passive element with two endpoint indices.
#[derive(Clone, Copy, Debug)]
pub struct PassiveBranch {
    from: usize,
    to: usize,
    element: PassiveElement,
}

impl PassiveBranch {
    /// Connect the element. Equal endpoints carry zero net nodal current.
    pub fn new(from: usize, to: usize, element: PassiveElement) -> Self {
        Self { from, to, element }
    }

    /// Node indices in positive-to-negative current orientation.
    pub fn endpoints(self) -> [usize; 2] {
        [self.from, self.to]
    }

    /// Oriented current from the original voltage drop and device admittance.
    pub fn current(
        self,
        frequency: Frequency,
        volts: &[ElectricPotential<Complex64>],
    ) -> Result<ElectricCurrent<Complex64>, StampError> {
        let voltage = |i: usize| {
            let z = volts.get(i).ok_or(StampError::Node(i))?.into_base();
            if !finite(z) {
                return Err(StampError::Boundary(i));
            }
            Ok(z)
        };
        let a = voltage(self.from)?;
        let b = voltage(self.to)?;
        check_frequency(frequency)?;
        if self.from == self.to {
            return Ok(ElectricCurrent::default());
        }
        let y = self.element.at(frequency)?;
        if y == Complex64::default() {
            return Ok(ElectricCurrent::default());
        }
        current(y, a - b).map(ElectricCurrent::from_base)
    }

    /// Add this element's free-node matrix and prescribed-voltage RHS terms.
    /// The boundary slice defines the node domain. A failed call emits no terms.
    /// Callers own additive storage and check accumulated sums for overflow.
    pub fn stamp(
        self,
        frequency: Frequency,
        boundary: &[Option<ElectricPotential<Complex64>>],
        matrix: impl FnMut(usize, usize, ElectricConductance<Complex64>),
        rhs: impl FnMut(usize, ElectricCurrent<Complex64>),
    ) -> Result<(), StampError> {
        let ends = [self.from, self.to];
        let mut fixed = [None; 2];
        for (slot, node) in fixed.iter_mut().zip(ends) {
            *slot = *boundary.get(node).ok_or(StampError::Node(node))?;
            if let Some(v) = slot
                && !finite(v.into_base())
            {
                return Err(StampError::Boundary(node));
            }
        }
        check_frequency(frequency)?;
        if self.from == self.to {
            return Ok(());
        }
        let y = self.element.at(frequency)?;
        if y == Complex64::default() || fixed.iter().all(Option::is_some) {
            return Ok(());
        }
        let branch = Branch {
            from: self.from,
            to: self.to,
            conductance: ElectricConductance::from_base(y),
        };
        // A branch has at most one eliminated neighbor. Validate its source
        // contribution before the shared kernel invokes either user's callback.
        if let Some(v) = fixed.into_iter().flatten().next() {
            current(y, v.into_base())?;
        }
        branch.stamp(fixed, matrix, rhs);
        Ok(())
    }
}

fn finite(z: Complex64) -> bool {
    z.re.is_finite() && z.im.is_finite()
}

fn current(y: Complex64, v: Complex64) -> Result<Complex64, StampError> {
    let current = y * v;
    // A pure susceptance exchanges the input quadratures. Each nonzero
    // input component must survive multiplication by a nonzero admittance.
    let (real_input, imag_input) = if y.im == 0. {
        (v.re, v.im)
    } else {
        (v.im, v.re)
    };
    if !finite(current)
        || (real_input != 0. && current.re == 0.)
        || (imag_input != 0. && current.im == 0.)
    {
        return Err(StampError::Range);
    }
    Ok(current)
}

fn check_frequency(f: Frequency) -> Result<(), StampError> {
    if !f.as_base().is_finite() || f.into_base() < 0. {
        Err(StampError::Frequency)
    } else {
        Ok(())
    }
}
