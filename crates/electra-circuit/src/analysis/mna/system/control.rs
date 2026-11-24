//! Compile each control into a dimensionless operator term once per topology.
use super::{Stamp, VOLT};
use crate::analysis::mna::MnaError;
use crate::{Circuit, ElectricCurrent, Element, SourceValue, device::Controlled};

#[derive(Clone, Copy)]
pub(super) enum Signal {
    Difference(usize, usize),
    Unknown(usize),
}
impl Signal {
    #[inline]
    pub fn read(self, x: &[f64]) -> f64 {
        match self {
            Self::Difference(a, b) => x[a] - x[b],
            Self::Unknown(s) => x[s],
        }
    }
    pub fn coefficients(self, mut add: impl FnMut(usize, f64)) {
        match self {
            Self::Difference(a, b) => {
                add(a, 1.);
                add(b, -1.);
            }
            Self::Unknown(s) => add(s, 1.),
        }
    }
}

pub(super) struct VoltageControl {
    pub row: usize,
    pub signal: Signal,
    pub output_weight: f64,
    pub input_weight: f64,
}

#[derive(Default)]
pub(super) struct Couplings {
    pub currents: Vec<(Stamp, Signal)>,
    pub voltages: Vec<VoltageControl>,
}

impl Controlled {
    fn unit_response(self, branches: &[(usize, ElectricCurrent)]) -> Result<SourceValue, MnaError> {
        let (response, active) = match self {
            Self::VoltageGain([a, b], _) if a == b => {
                (SourceValue::Voltage(Default::default()), false)
            }
            Self::Transconductance([a, b], _) if a == b => {
                (SourceValue::Current(Default::default()), false)
            }
            Self::VoltageGain(_, k) => (SourceValue::Voltage(VOLT * k), k.into_base() != 0.),
            Self::Transconductance(_, g) => (SourceValue::Current(g * VOLT), g.into_base() != 0.),
            Self::CurrentGain(s, k) => {
                (SourceValue::Current(branches[s].1 * k), k.into_base() != 0.)
            }
            Self::Transresistance(s, r) => {
                (SourceValue::Voltage(r * branches[s].1), r.into_base() != 0.)
            }
        };
        let value = match response {
            SourceValue::Voltage(v) => v.into_base(),
            SourceValue::Current(i) => i.into_base(),
        };
        if !value.is_finite() || (active && value == 0.) {
            return Err(MnaError::Arithmetic);
        }
        Ok(response)
    }
    fn signal(self, nodes: usize) -> Signal {
        match self {
            Self::VoltageGain([a, b], _) | Self::Transconductance([a, b], _) => {
                Signal::Difference(a, b)
            }
            Self::CurrentGain(s, _) | Self::Transresistance(s, _) => Signal::Unknown(nodes + s),
        }
    }
}

impl Couplings {
    pub fn prepare(
        c: &Circuit,
        units: &mut [ElectricCurrent],
        branches: &[(usize, ElectricCurrent)],
    ) -> Result<Self, MnaError> {
        if !c.requires_mna() {
            return Ok(Self::default());
        }
        let voltage_count = c.controlled.iter().filter(|s| s.voltage_output()).count();
        let mut result = Self {
            currents: Vec::with_capacity(c.controlled.len() - voltage_count),
            voltages: Vec::with_capacity(voltage_count),
        };
        // Include controlled injections in the physical KCL row scales.
        let controls = c.parts.iter().filter_map(|p| match p.element {
            Element::Controlled(i) => Some((p, c.controlled[i])),
            _ => None,
        });
        for (p, law) in controls {
            if let SourceValue::Current(i) = law.unit_response(branches)? {
                for n in [p.positive, p.negative] {
                    if n != 0 {
                        units[n] = units[n].max(i.abs());
                    }
                }
            }
        }
        for u in &mut units[1..] {
            if *u == ElectricCurrent::default() {
                *u = ElectricCurrent::from_base(1.);
            }
        }
        let mut branch = 0;
        for p in &c.parts {
            if let Element::Controlled(i) = p.element {
                let law = c.controlled[i];
                let signal = law.signal(units.len());
                match law.unit_response(branches)? {
                    SourceValue::Current(i) if i == ElectricCurrent::default() => (),
                    SourceValue::Voltage(v) if v == Default::default() => (),
                    SourceValue::Current(i) => result
                        .currents
                        .push((Stamp::new(p.positive, p.negative, i, units)?, signal)),
                    SourceValue::Voltage(v) => {
                        let gain = (v / VOLT).into_base();
                        let unit = gain.abs().max(1.);
                        result.voltages.push(VoltageControl {
                            row: units.len() + branch,
                            signal,
                            output_weight: 1. / unit,
                            input_weight: gain / unit,
                        });
                    }
                }
            }
            branch += usize::from(c.voltage_output(p));
        }
        Ok(result)
    }
}
