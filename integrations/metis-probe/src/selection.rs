//! Metis selection for an ordered list of schematic pin rows.
//! Hosts pass local pixel coordinates and the key decoded by Metis.
use electra_schematic::PinId;
use metis_core::input::{Key, PointerDrag, PointerDragError};
use std::borrow::Cow;

/// Focus and press capture over a borrowed list of visible pin rows.
/// Rows begin at local y=0 and have equal positive height.
pub struct PinList<'a> {
    rows: Cow<'a, [PinId]>,
    width: f64,
    height: f64,
    focus: Option<usize>,
    press: Option<(usize, PointerDrag)>,
}

impl<'a> PinList<'a> {
    /// Construct a hit-testable region with a finite width and row height.
    pub fn new(rows: &'a [PinId], width: f64, row_height: f64) -> Option<Self> {
        (width.is_finite() && width > 0.0 && row_height.is_finite() && row_height > 0.0).then_some(
            Self {
                rows: Cow::Borrowed(rows),
                width,
                height: row_height,
                focus: None,
                press: None,
            },
        )
    }

    fn hit(&self, [x, y]: [f64; 2]) -> Option<usize> {
        if !(0.0..self.width).contains(&x) || !y.is_finite() || y < 0.0 {
            return None;
        }
        let row = (y / self.height).floor() as usize;
        (row < self.rows.len()).then_some(row)
    }

    /// Start primary pointer capture. A new press cancels any older capture.
    pub fn press(&mut self, point: [f64; 2]) -> Result<(), PointerDragError> {
        self.press = None;
        let drag = PointerDrag::try_new(point, [1.0, 1.0])?;
        self.press = self.hit(point).map(|row| (row, drag));
        Ok(())
    }

    /// Commit a click released on the pressed row within four pixels of origin.
    pub fn release(&mut self, point: [f64; 2]) -> Result<(), PointerDragError> {
        if let Some((row, drag)) = self.press.take() {
            let [dx, dy] = drag.displacement(point)?;
            if dx.hypot(dy) <= 4.0 && self.hit(point) == Some(row) {
                self.focus = Some(row);
            }
        }
        Ok(())
    }

    /// Cancel capture on focus loss or pointer cancellation.
    pub fn cancel_press(&mut self) {
        self.press = None;
    }

    /// Handle list navigation. Returns true when the list consumes the key.
    pub fn key(&mut self, key: Key) -> bool {
        let Some(last) = self.rows.len().checked_sub(1) else {
            return false;
        };
        self.focus = match key {
            Key::Home => Some(0),
            Key::End => Some(last),
            Key::ArrowDown => Some(self.focus.map_or(0, |i| (i + 1).min(last))),
            Key::ArrowUp => Some(self.focus.map_or(last, |i| i.saturating_sub(1))),
            Key::Escape => None,
            _ => return false,
        };
        self.cancel_press();
        true
    }

    /// Return the stable occurrence and pin identity for the focused row.
    pub fn selected(&self) -> Option<PinId> {
        self.focus.map(|i| self.rows[i])
    }
}

impl PinList<'_> {
    /// Install a rebuilt row list after placement or opening a document.
    pub fn replace_rows(&mut self, rows: Vec<PinId>) {
        let old = self.selected();
        self.focus = old.and_then(|id| rows.iter().position(|item| *item == id));
        self.rows = Cow::Owned(rows);
        self.cancel_press();
    }
    /// Borrow displayed rows in navigation order.
    pub fn rows(&self) -> &[PinId] {
        &self.rows
    }
}
