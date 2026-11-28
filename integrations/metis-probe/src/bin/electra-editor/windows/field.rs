use electra_metis_probe::overlay::RailMap;
use metis_platform::{
    Color, Framebuffer, draw_line,
    typeface::{TextStyle, draw_text},
};

pub(super) struct Field {
    map: RailMap,
    pixels: i32,
    thermal: bool,
}
const CORNER: [f64; 2] = [380., 320.];
impl Field {
    pub(super) fn load(path: &std::path::Path, mode: &str, zoom: &str) -> Result<Self, String> {
        let thermal = match mode {
            "thermal" => true,
            "drop" => false,
            _ => return Err("use field drop or field thermal".into()),
        };
        let pixels = match zoom {
            "1" => 24,
            "2" => 48,
            _ => return Err("field zoom must be 1 or 2".into()),
        };
        let text =
            std::fs::read_to_string(path.with_extension("rail")).map_err(|e| e.to_string())?;
        let map = RailMap::parse(&text).ok_or("invalid rail map")?;
        Ok(Self {
            map,
            pixels,
            thermal,
        })
    }
    pub(super) fn read(&self, x: i32, y: i32) -> Option<String> {
        let s = self
            .map
            .view()
            .probe([x.into(), y.into()], CORNER, self.pixels.into())?;
        Some(format!(
            "net {} x={} mm y={} mm drop={:.12} V rise={:.12} K",
            s.net.0,
            s.center[0].into_base() * 1e3,
            s.center[1].into_base() * 1e3,
            s.drop.into_base(),
            s.rise.into_base()
        ))
    }
    pub(super) fn draw(&self, frame: &mut Framebuffer, style: TextStyle) {
        let [rows, cols] = self.map.shape();
        let view = self.map.view();
        let title = if self.thermal {
            "Temperature rise (K)"
        } else {
            "Rail drop (V)"
        };
        draw_text(frame, 380, 278, title, style);
        let width = frame.width() as i32;
        let height = frame.height() as i32 - 90;
        let cell = self.pixels;
        let rows = rows.min((height - 320).max(0).div_euclid(cell) as usize);
        let cols = cols.min((width - 380).max(0).div_euclid(cell) as usize);
        for row in 0..rows {
            for col in 0..cols {
                let x = 380 + col as i32 * cell;
                let y = 320 + row as i32 * cell;
                let s = view
                    .probe([(x + 1).into(), (y + 1).into()], CORNER, cell.into())
                    .expect("visible cell");
                let level = if self.thermal {
                    s.rise.into_base() / 50.
                } else {
                    s.drop.into_base().abs() / 0.5
                };
                let intensity = (level.clamp(0., 1.) * 223.) as u8 + 32;
                let color = if self.thermal {
                    Color::rgba(intensity, 48, 42, 255)
                } else {
                    Color::rgba(35, 75, intensity, 255)
                };
                for line in y..y + cell - 1 {
                    draw_line(frame, (x, line), (x + cell - 2, line), color);
                }
            }
        }
    }
}
