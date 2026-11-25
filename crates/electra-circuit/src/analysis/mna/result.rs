use super::{
    MnaError, MnaOptions,
    system::{System, VOLT},
};
use crate::{
    Circuit, DcResult, ElectricCurrent, ElectricPotential, Element, SourceValue,
    analysis::excitation::{CurrentSum, Excitation, voltage},
};

/// Original physical probes and independently checked source-constraint residual.
#[derive(Debug)]
pub struct MnaPoint<'a> {
    /// Voltages and signed currents. Relaxation sweeps are zero for this backend.
    pub probes: DcResult<'a>,
    /// Largest absolute difference between an ideal source's imposed and measured voltage.
    pub residual_volts: ElectricPotential,
    /// Arnoldi iterations for this point. Direct factorizations report zero.
    pub iterations: usize,
}

#[derive(Default, Clone, Copy)]
pub(super) struct Balance {
    net: CurrentSum,
    scale: ElectricCurrent,
}
impl<'a> MnaPoint<'a> {
    pub(super) fn new(circuit: &'a Circuit) -> Self {
        Self {
            probes: DcResult::new(circuit),
            iterations: 0,
            residual_volts: ElectricPotential::default(),
        }
    }

    pub(super) fn project(
        &mut self,
        c: &Circuit,
        system: &System,
        x: &[f64],
        excitation: Excitation,
        limits: MnaOptions,
        balance: &mut [Balance],
    ) -> Result<(), MnaError> {
        if !x.iter().all(|v| v.is_finite()) {
            return Err(MnaError::Arithmetic);
        }
        balance.fill(Balance::default());
        for ((_, v), &value) in self.probes.volts.iter_mut().zip(x) {
            *v = VOLT * value;
        }
        let mut sources = system.sources.iter().enumerate();
        self.residual_volts = ElectricPotential::default();
        for (i, (p, (_, out))) in c.parts.iter().zip(&mut self.probes.amperes).enumerate() {
            let potential = |n: usize| self.probes.volts[n].1;
            let specified = match p.element {
                Element::Controlled(index) => c.controlled[index].response(potential, |branch| {
                    system.sources[branch].current_unit * x[system.node_count() + branch]
                }),
                _ => match p.passive_current(c, potential, i, excitation) {
                    Some(current) => SourceValue::Current(current),
                    None => SourceValue::Voltage(voltage(p, i, excitation)),
                },
            };
            let current = match specified {
                SourceValue::Current(value) => value,
                SourceValue::Voltage(target) => {
                    let (j, s) = sources.next().expect("compiled voltage source slot");
                    let measured = potential(p.positive) - potential(p.negative);
                    let error = (measured - target).abs();
                    let limit = limits.voltage_absolute + target.abs() * limits.relative;
                    if !error.as_base().is_finite() || !limit.as_base().is_finite() {
                        return Err(MnaError::Arithmetic);
                    }
                    if error > limit {
                        return Err(MnaError::SourceVoltage {
                            part: i,
                            residual: error,
                            limit,
                        });
                    }
                    if matches!(p.element, Element::Voltage { .. })
                        && measured == ElectricPotential::default()
                        && target != ElectricPotential::default()
                    {
                        return Err(MnaError::Resolution { part: i });
                    }
                    self.residual_volts = self.residual_volts.max(error);
                    s.current_unit * x[system.node_count() + j]
                }
            };
            if !current.as_base().is_finite() {
                return Err(MnaError::Arithmetic);
            }
            *out = current;
            for (n, delta) in [(p.positive, current), (p.negative, -current)] {
                balance[n].net.add(delta);
                balance[n].scale += current.abs();
            }
        }
        let mut max = ElectricCurrent::default();
        for (n, b) in balance.iter().enumerate() {
            let error = b.net.value().abs();
            let limit = limits.current_absolute + b.scale * limits.relative;
            if !b.scale.as_base().is_finite()
                || !limit.as_base().is_finite()
                || !error.as_base().is_finite()
            {
                return Err(MnaError::Arithmetic);
            }
            if error > limit {
                return Err(MnaError::CurrentBalance {
                    node: n,
                    residual: error,
                    limit,
                });
            }
            max = max.max(error);
        }
        self.probes.residual_amperes = max;
        Ok(())
    }
}
