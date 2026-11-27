//! Pin-aware model assignment for placed component occurrences.
use super::{PinId, Schematic};
use std::{collections::BTreeSet, sync::Arc};

/// Model library key and symbol pins in the model's terminal order.
#[derive(Debug, PartialEq, Eq)]
pub struct ModelBinding {
    name: String,
    pins: Vec<u32>,
}
impl ModelBinding {
    /// Borrow the model library key.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Borrow symbol pin numbers in model terminal order.
    pub fn pins(&self) -> &[u32] {
        &self.pins
    }

    /// Construct a reusable model definition with unique ordered terminal pins.
    pub fn new(name: String, pins: Vec<u32>) -> Result<Self, &'static str> {
        if name.is_empty() || name.len() > usize::from(u16::MAX) {
            return Err("invalid model name length");
        }
        if pins.is_empty() || pins.len() > usize::from(u16::MAX) {
            return Err("invalid model terminal count");
        }
        let mut seen = BTreeSet::new();
        for &pin in &pins {
            if !seen.insert(pin) {
                return Err("duplicate model terminal pin");
            }
        }
        Ok(Self { name, pins })
    }
}
impl Schematic {
    /// Assign or replace a model. Each ordered terminal must map to a distinct
    /// existing pin. Validation errors preserve the previous assignment.
    pub fn bind_model(
        &mut self,
        instance: u64,
        name: String,
        pins: Vec<u32>,
    ) -> Result<(), &'static str> {
        self.replace_model(instance, Some(Arc::new(ModelBinding::new(name, pins)?)))
            .map(|_| ())
    }

    /// Exchange a shared model assignment. Validate all referenced occurrence
    /// pins before returning the previous binding for undo or reuse.
    pub fn replace_model(
        &mut self,
        instance: u64,
        model: Option<Arc<ModelBinding>>,
    ) -> Result<Option<Arc<ModelBinding>>, &'static str> {
        if let Some(model) = model {
            if model
                .pins()
                .iter()
                .any(|&pin| !self.pins.contains_key(&PinId { instance, pin }))
            {
                return Err("model terminal references missing pin");
            }
            Ok(self.models.insert(instance, model))
        } else {
            Ok(self.models.remove(&instance))
        }
    }
    /// Inspect the model assigned to one component occurrence.
    pub fn model(&self, instance: u64) -> Option<&ModelBinding> {
        self.models.get(&instance).map(AsRef::as_ref)
    }
}
