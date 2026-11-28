//! Zero-copy Iris fields with Metis pointer-to-board readback.
use electra_ac::{ElectricPotential, Length, TemperatureDifference};
use electra_board::board::NetId;
use iris::view::ScalarFieldView;
use metis_core::input::PointerDrag;

/// Registered cell-centred fields, row-major with x increasing first.
/// Origin is the low corner of cell (0,0) and board y increases down the displayed field.
pub struct Overlay<'a> {
    volts: ScalarFieldView<'a, ElectricPotential, 2>,
    rise: ScalarFieldView<'a, TemperatureDifference, 2>,
    nets: &'a [NetId],
    origin: [Length; 2],
    pitch: Length,
}
/// The exact solver cell selected in board coordinates.
#[derive(Debug, Clone, Copy)]
pub struct Probe {
    /// Cell centre in metres.
    pub center: [Length; 2],
    /// Stable board net ID.
    pub net: NetId,
    /// Rail voltage loss in volts.
    pub drop: ElectricPotential,
    /// Temperature rise in kelvin.
    pub rise: TemperatureDifference,
}
impl<'a> Overlay<'a> {
    /// Bind two solver arrays and net identities to a regular physical grid.
    pub fn new(
        shape: [usize; 2],
        origin: [Length; 2],
        pitch: Length,
        nets: &'a [NetId],
        volts: &'a [ElectricPotential],
        rise: &'a [TemperatureDifference],
    ) -> Option<Self> {
        if nets.len() != volts.len()
            || !origin.iter().all(|x| x.as_base().is_finite())
            || !(pitch.into_base().is_finite() && pitch.into_base() > 0.)
        {
            return None;
        }
        Some(Self {
            volts: ScalarFieldView::new(volts, shape).ok()?,
            rise: ScalarFieldView::new(rise, shape).ok()?,
            nets,
            origin,
            pitch,
        })
    }
    /// Resolve a Metis client-pixel pointer relative to the low corner of the field.
    pub fn probe(&self, pixel: [f64; 2], corner: [f64; 2], cell_pixels: f64) -> Option<Probe> {
        let scaled = PointerDrag::try_new(corner, [cell_pixels.recip(); 2])
            .ok()?
            .displacement(pixel)
            .ok()?;
        let [rows, cols] = *self.volts.extents();
        if scaled[0] < 0. || scaled[1] < 0. || scaled[0] >= cols as f64 || scaled[1] >= rows as f64
        {
            return None;
        }
        let [x, y] = scaled.map(|v| v.floor() as usize);
        let index = y * cols + x;
        Some(Probe {
            center: [
                self.origin[0] + self.pitch * (x as f64 + 0.5),
                self.origin[1] + self.pitch * (y as f64 + 0.5),
            ],
            net: self.nets[index],
            drop: self.volts.as_slice()[index],
            rise: self.rise.as_slice()[index],
        })
    }
}

mod map;
pub use map::RailMap;
