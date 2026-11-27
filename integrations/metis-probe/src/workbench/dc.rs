//! Compile supported pin-aware model bindings and probe a real DC sweep.
use super::*;
use electra_circuit::{ElectricCurrent, ElectricPotential, SourceValue};
use iris::view::{Axis, SeriesView};
use std::fmt::Write;

/// Iris views of the two signed physical quantities on their shared drive axis.
pub struct DcCurves<'a> {
    /// Sweep coordinate for both series.
    pub drive: Axis<'static>,
    /// Selected net voltage in volts.
    pub voltage: SeriesView<'a, ElectricPotential, ElectricPotential>,
    /// Selected terminal current in amperes.
    pub current: SeriesView<'a, ElectricPotential, ElectricCurrent>,
}

pub(super) fn part(name: &str) -> Result<(char, f64), String> {
    match name {
        "V2" => Ok(('V', 2.)),
        "R1k" => Ok(('R', 1000.)),
        "R2k" => Ok(('R', 2000.)),
        _ => Err(format!("unsupported DC model {name}")),
    }
}

pub(super) fn erc(model: &Schematic) -> Vec<electra_schematic::Diagnostic> {
    let mut issues = Vec::new();
    model.check(|d| issues.push(d));
    issues
}

pub(super) fn deck(model: &Schematic) -> Result<String, String> {
    let errors = erc(model);
    if !errors.is_empty() {
        return Err(format!("ERC {errors:?}"));
    }
    let mut text = String::new();
    let mut previous = None;
    let mut sweep = None;
    for (id, _) in model.pins() {
        if previous == Some(id.instance) {
            continue;
        }
        previous = Some(id.instance);
        let binding = model
            .model(id.instance)
            .ok_or_else(|| format!("missing model on {}", id.instance))?;
        let [a, b] = binding.pins() else {
            return Err("DC model requires two ordered pins".into());
        };
        let node = |pin| {
            model
                .probe_pin(PinId {
                    instance: id.instance,
                    pin,
                })
                .map(|(net, _)| net.0)
                .ok_or("model terminal is unwired")
        };
        let (kind, value) = part(binding.name())?;
        writeln!(
            text,
            "{kind}{} {} {} {value}",
            id.instance,
            node(*a)?,
            node(*b)?
        )
        .expect("string write");
        if kind == 'V' {
            sweep.get_or_insert(id.instance);
        }
    }
    writeln!(
        text,
        ".dc V{} 1 3 1",
        sweep.ok_or("no voltage source to sweep")?
    )
    .expect("string write");
    Ok(text)
}

#[derive(Default)]
pub(super) struct Trace {
    drive: Vec<ElectricPotential>,
    volts: Vec<ElectricPotential>,
    amps: Vec<ElectricCurrent>,
    label: String,
}
impl Trace {
    pub(super) fn solve(model: &Schematic, pin: PinId) -> Result<Self, String> {
        let (net, label) = model.probe_pin(pin).ok_or("select a wired pin")?;
        let circuit = electra_circuit::parse(&deck(model)?).map_err(|e| e.to_string())?;
        let mut sweep = circuit
            .dc_sweep(Default::default())
            .map_err(|e| e.to_string())?;
        let binding = model.model(pin.instance).ok_or("missing model")?;
        let branch = format!("{}{}", part(binding.name())?.0, pin.instance);
        let negative = match binding.pins().iter().position(|n| *n == pin.pin) {
            Some(0) => false,
            Some(1) => true,
            _ => return Err("pin is outside model".into()),
        };
        let node = net.0.to_string();
        let mut trace = Self {
            label: label.unwrap_or(&node).to_owned(),
            ..Default::default()
        };
        while let Some(point) = sweep.next_point().map_err(|e| e.to_string())? {
            let SourceValue::Voltage(drive) = point.excitation else {
                return Err("unexpected sweep units".into());
            };
            let v = point
                .result
                .volts
                .iter()
                .find(|(n, _)| *n == node)
                .ok_or("missing output net")?
                .1;
            let i = point
                .result
                .amperes
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(&branch))
                .ok_or("missing branch current")?
                .1;
            trace.drive.push(drive);
            trace.volts.push(v);
            trace.amps.push(if negative { -i } else { i });
        }
        Ok(trace)
    }
    pub(super) fn curves(&self) -> Result<DcCurves<'_>, String> {
        Ok(DcCurves {
            drive: Axis::with_unit("drive", "V"),
            voltage: SeriesView::new(self.label.as_str(), &self.drive, &self.volts)
                .map_err(|e| e.to_string())?,
            current: SeriesView::new(self.label.as_str(), &self.drive, &self.amps)
                .map_err(|e| e.to_string())?,
        })
    }
    pub(super) fn cursor(&self, index: usize) -> Result<String, String> {
        let curves = self.curves()?;
        let (x, y) = curves
            .voltage
            .sample_at(index)
            .ok_or("cursor outside trace")?;
        let (_, current) = curves
            .current
            .sample_at(index)
            .ok_or("cursor outside trace")?;
        Ok(format!(
            "{} {}={} {} voltage={} V current={} A",
            curves.voltage.name(),
            curves.drive.label(),
            x.into_base(),
            curves.drive.unit().unwrap_or(""),
            y.into_base(),
            current.into_base()
        ))
    }
}
