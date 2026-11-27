//! Cross-probe a schematic occurrence against frequency-domain solver traces.
use electra_ac::{ElectricCurrent, ElectricPotential, Frequency};
use electra_schematic::{NetId, PinId, Schematic};
use iris::{IrisResult, view::SeriesView};
use std::collections::BTreeMap;

/// Borrowed result columns sharing one frequency axis. Voltage is net-relative
/// to the solver reference. Current is signed into the selected component pin.
pub struct ProbeTable<'a> {
    /// Frequencies in solver sample order.
    pub frequencies: &'a [Frequency],
    /// Node voltage columns keyed by electrical identity.
    pub voltages: BTreeMap<NetId, &'a [ElectricPotential]>,
    /// Terminal current columns keyed by placed occurrence and local pin.
    pub currents: BTreeMap<PinId, &'a [ElectricCurrent]>,
}

/// Available Iris series for a single wired pin.
pub struct PinTraces<'a> {
    /// Stable electrical identity independent of the displayed label.
    pub net: NetId,
    /// Node voltage when the solver recorded this net.
    pub voltage: Option<SeriesView<'a, Frequency, ElectricPotential>>,
    /// Terminal current when the solver recorded this occurrence and pin.
    pub current: Option<SeriesView<'a, Frequency, ElectricCurrent>>,
}

impl<'a> ProbeTable<'a> {
    /// Resolve a selection after editing or reopening. Missing and unwired pins
    /// return None. Each available series borrows the existing samples and label.
    pub fn select(
        &self,
        schematic: &'a Schematic,
        pin: PinId,
    ) -> IrisResult<Option<PinTraces<'a>>> {
        let Some((net, label)) = schematic.probe_pin(pin) else {
            return Ok(None);
        };
        let voltage = self
            .voltages
            .get(&net)
            .map(|&values| {
                SeriesView::new(label.unwrap_or("Net voltage"), self.frequencies, values)
            })
            .transpose()?;
        let current = self
            .currents
            .get(&pin)
            .map(|&values| {
                SeriesView::new(label.unwrap_or("Pin current"), self.frequencies, values)
            })
            .transpose()?;
        Ok(Some(PinTraces {
            net,
            voltage,
            current,
        }))
    }
}
