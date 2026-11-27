//! Reversible edits on the selected occurrence. History owns only prior values.
use crate::PinList;
use electra_schematic::{ModelBinding, NetId, PinId, Schematic};
use metis_core::input::{Accelerator, Key, Modifiers};
use std::sync::Arc;

/// Reason a selected edit could not be committed.
#[derive(Debug, PartialEq, Eq)]
pub enum EditError {
    /// No pin is selected or the selected pin is absent from the project.
    NoPin,
    /// A label edit requires a wired terminal.
    Unwired,
    /// The schematic rejected a model definition or label.
    Invalid(&'static str),
}

enum Change {
    Wire(PinId, Option<NetId>),
    Label(NetId, Option<String>),
    Model(u64, Option<Arc<ModelBinding>>),
}
impl Change {
    fn apply(self, model: &mut Schematic) -> Result<Self, EditError> {
        Ok(match self {
            Self::Wire(id, net) => {
                let pin = model.pin_mut(id).ok_or(EditError::NoPin)?;
                Self::Wire(id, std::mem::replace(&mut pin.net, net))
            }
            Self::Label(net, label) => Self::Label(
                net,
                match label {
                    Some(text) => model.set_label(net, text).map_err(EditError::Invalid)?,
                    None => model.remove_label(net),
                },
            ),
            Self::Model(id, binding) => Self::Model(
                id,
                model
                    .replace_model(id, binding)
                    .map_err(EditError::Invalid)?,
            ),
        })
    }
}

/// Exclusive editing session. Failed edits preserve both history stacks.
pub struct PinEditor<'a> {
    model: &'a mut Schematic,
    undo: Vec<Change>,
    redo: Vec<Change>,
}
impl<'a> PinEditor<'a> {
    /// Start an editing session for a new or reopened project.
    pub fn new(model: &'a mut Schematic) -> Self {
        Self {
            model,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }
    /// Borrow the current project for saving or probing.
    pub fn schematic(&self) -> &Schematic {
        self.model
    }
    fn commit(&mut self, change: Change) -> Result<(), EditError> {
        let inverse = change.apply(self.model)?;
        self.undo.push(inverse);
        self.redo.clear();
        Ok(())
    }
    /// Connect or disconnect the focused pin.
    pub fn wire(&mut self, list: &PinList<'_>, net: Option<NetId>) -> Result<(), EditError> {
        let id = list.selected().ok_or(EditError::NoPin)?;
        self.commit(Change::Wire(id, net))
    }
    /// Set or remove the focused net's display label.
    pub fn label(&mut self, list: &PinList<'_>, text: Option<String>) -> Result<(), EditError> {
        let id = list.selected().ok_or(EditError::NoPin)?;
        let net = self
            .model
            .pin_mut(id)
            .ok_or(EditError::NoPin)?
            .net
            .ok_or(EditError::Unwired)?;
        self.commit(Change::Label(net, text))
    }
    /// Set or remove the model on the focused pin's component.
    pub fn bind(
        &mut self,
        list: &PinList<'_>,
        binding: Option<Arc<ModelBinding>>,
    ) -> Result<(), EditError> {
        let id = list.selected().ok_or(EditError::NoPin)?;
        self.model.pin_mut(id).ok_or(EditError::NoPin)?;
        self.commit(Change::Model(id.instance, binding))
    }
    /// Handle the host primary Z (undo) or primary Shift Z (redo) accelerator.
    /// Returns true when an edit was replayed.
    pub fn shortcut(&mut self, key: Accelerator) -> bool {
        if !matches!(key.key(),Key::Letter(letter) if letter.byte()==b'Z') {
            return false;
        }
        let primary = Modifiers::host_primary();
        let (from, to) = if key.modifiers() == primary {
            (&mut self.undo, &mut self.redo)
        } else if key.modifiers() == primary.union(Modifiers::SHIFT) {
            (&mut self.redo, &mut self.undo)
        } else {
            return false;
        };
        let Some(change) = from.pop() else {
            return false;
        };
        // Exclusive access preserves every pin and validated value held in history.
        to.push(change.apply(self.model).expect("valid inverse edit"));
        true
    }
}
