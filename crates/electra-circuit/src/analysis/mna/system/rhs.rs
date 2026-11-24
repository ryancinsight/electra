use super::{System, VOLT, normalized};
use crate::analysis::excitation::CurrentSum;
use crate::{Circuit, ElectricCurrent, Element, SourceValue, analysis::mna::MnaError};

// Store the nominal right-hand side without the swept source. This preserves
// small fixed injections when a large nominal source is replaced by zero.
pub(in crate::analysis::mna) enum Drive {
    Bias,
    Current {
        ports: [(usize, CurrentSum, ElectricCurrent); 2],
    },
    Voltage {
        slot: usize,
    },
}
impl System {
    pub fn rhs(
        &self,
        c: &Circuit,
        swept: Option<usize>,
        rhs: &mut [f64],
    ) -> Result<Drive, MnaError> {
        let injections = crate::analysis::excitation::injections(c, self.units.len(), swept, |n| n);
        let mut drive = Drive::Bias;
        let mut driven_nodes = [0, 0];
        if let Some(i) = swept
            && matches!(c.parts[i].element, Element::Current { .. })
        {
            let p = &c.parts[i];
            driven_nodes = [p.positive, p.negative];
            drive = Drive::Current {
                ports: driven_nodes.map(|n| (n, injections[n], self.units[n])),
            };
        }
        for (n, ((v, &i), &unit)) in rhs
            .iter_mut()
            .zip(&injections)
            .zip(&self.units)
            .enumerate()
            .skip(1)
        {
            if !driven_nodes.contains(&n) {
                *v = normalized(i.value(), unit)?;
            }
        }
        for (j, s) in self.sources.iter().enumerate() {
            let slot = self.units.len() + j;
            if Some(s.part) == swept {
                drive = Drive::Voltage { slot };
            } else if let Element::Voltage { volts } = c.parts[s.part].element {
                rhs[slot] = (volts / VOLT).into_base();
            }
        }
        Ok(drive)
    }
}
impl Drive {
    pub fn update(&self, x: SourceValue, rhs: &mut [f64]) -> Result<(), MnaError> {
        match (self, x) {
            (Self::Voltage { slot }, SourceValue::Voltage(v)) => {
                rhs[*slot] = (v / VOLT).into_base()
            }
            (Self::Current { ports }, SourceValue::Current(i)) => {
                for (&(n, base, unit), delta) in ports.iter().zip([-i, i]) {
                    if n != 0 {
                        rhs[n] = normalized(base.with(delta), unit)?;
                    }
                }
            }
            _ => return Err(MnaError::Backend("excitation dimension mismatch")),
        }
        Ok(())
    }
}
