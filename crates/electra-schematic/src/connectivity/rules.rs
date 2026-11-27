use super::{NetId, PinId, Role, Schematic};
use std::collections::BTreeMap;

/// Actionable electrical-rule finding with exact occurrence identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Diagnostic {
    /// Required terminal has no distinct peer on its assigned net.
    Unconnected(PinId),
    /// An additional push-pull output drives the same net as the first.
    OutputConflict(NetId, PinId, PinId),
}

impl Schematic {
    /// Emit diagnostics in pin order. Uses one summary per connected net
    /// and streams findings directly to the caller without cloning the schematic.
    pub fn check(&self, mut report: impl FnMut(Diagnostic)) {
        #[derive(Default)]
        struct Summary {
            peers: usize,
            driver: Option<PinId>,
        }
        let mut nets = BTreeMap::<NetId, Summary>::new();
        for (id, pin) in &self.pins {
            if let Some(net) = pin.net {
                let summary = nets.entry(net).or_default();
                summary.peers += 1;
                if pin.role == Role::Output {
                    summary.driver.get_or_insert(*id);
                }
            }
        }
        for (id, pin) in &self.pins {
            let binding = pin.net.map(|net| (net, &nets[&net]));
            if pin.required && binding.is_none_or(|(_, summary)| summary.peers < 2) {
                report(Diagnostic::Unconnected(*id));
            }
            if pin.role == Role::Output
                && let Some((net, summary)) = binding
                && let Some(first) = summary.driver
                && first != *id
            {
                report(Diagnostic::OutputConflict(net, first, *id));
            }
        }
    }
}
