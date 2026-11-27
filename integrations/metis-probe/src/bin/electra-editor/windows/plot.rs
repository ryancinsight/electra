//! Render the two borrowed Iris series into separate unit-scaled Metis panels.
use electra_circuit::ElectricPotential;
use electra_metis_probe::workbench::DcCurves;
use iris::view::SeriesView;
use metis_platform::{
    Color, Framebuffer, draw_line,
    typeface::{TextStyle, draw_text},
};

pub(super) fn draw(frame: &mut Framebuffer, curves: &DcCurves<'_>, style: TextStyle) {
    let width = (frame.width() as i32 - 398) / 2;
    let bottom = frame.height() as i32 - 115;
    if width < 130 || bottom < 350 {
        return;
    }
    let axis = format!(
        "{} ({})",
        curves.drive.label(),
        curves.drive.unit().unwrap_or("")
    );
    for (left, title) in [(380, "Voltage (V)"), (380 + width, "Current (A)")] {
        draw_text(frame, left, 275, title, style);
        draw_text(frame, left, bottom + 6, &axis, style);
    }
    panel(
        frame,
        &curves.voltage,
        |v| v.into_base(),
        380,
        width,
        bottom,
        style,
    );
    panel(
        frame,
        &curves.current,
        |a| a.into_base(),
        380 + width,
        width,
        bottom,
        style,
    );
}

fn panel<Y>(
    frame: &mut Framebuffer,
    series: &SeriesView<'_, ElectricPotential, Y>,
    value: impl Fn(&Y) -> f64,
    left: i32,
    width: i32,
    bottom: i32,
    style: TextStyle,
) {
    let Some((first, _)) = series.sample_at(0) else {
        return;
    };
    let x0 = first.into_base();
    let x1 = series
        .coordinates()
        .last()
        .expect("nonempty drive")
        .into_base();
    let (low, high) = series
        .samples()
        .iter()
        .map(&value)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), y| {
            (a.min(y), b.max(y))
        });
    draw_text(frame, left, 297, &format!("{low:.3e} .. {high:.3e}"), style);
    let top = 322;
    let right = left + width - 12;
    draw_line(frame, (left, top), (left, bottom), Color::GRAY);
    draw_line(frame, (left, bottom), (right, bottom), Color::GRAY);
    let mut previous = None;
    for (x, y) in series.iter() {
        let dx = if x1 == x0 {
            0.5
        } else {
            (x.into_base() - x0) / (x1 - x0)
        };
        let dy = if high == low {
            0.5
        } else {
            (value(y) - low) / (high - low)
        };
        let point = (
            left + (dx * f64::from(right - left)).round() as i32,
            bottom - (dy * f64::from(bottom - top)).round() as i32,
        );
        if let Some(start) = previous {
            draw_line(frame, start, point, Color::GREEN);
        } else {
            frame.set_pixel(point.0, point.1, Color::GREEN);
        }
        previous = Some(point);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use electra_circuit::ElectricCurrent;
    use iris::view::Axis;
    use metis_platform::typeface::TextSize;

    #[test]
    fn native_frame_draws_voltage_peak_and_constant_signed_current_from_iris_samples() {
        let x = [1., 2., 3.].map(ElectricPotential::from_base);
        let v = [0., 2., 0.].map(ElectricPotential::from_base);
        let i = [-0.001; 3].map(ElectricCurrent::from_base);
        let curves = DcCurves {
            drive: Axis::with_unit("drive", "V"),
            voltage: SeriesView::new("TAP", &x, &v).unwrap(),
            current: SeriesView::new("TAP", &x, &i).unwrap(),
        };
        let mut frame = Framebuffer::new(960, 650).unwrap();
        let style = TextStyle::new(Color::WHITE, TextSize::new(15.).unwrap());
        draw(&mut frame, &curves, style);
        for (x, y) in [
            (380, 535),
            (515, 322),
            (649, 535),
            (661, 428),
            (795, 428),
            (930, 428),
        ] {
            assert_eq!(frame.get_pixel(x, y), Color::GREEN, "sample pixel {x},{y}");
        }
        assert_ne!(frame.get_pixel(515, 428), Color::GREEN);
    }
}
