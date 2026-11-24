//! An affine initial guess from two verified solutions of the same linear system.
use crate::SourceValue;

#[derive(Default)]
enum Stage {
    #[default]
    Empty,
    First(SourceValue),
    Second(SourceValue, SourceValue),
    Ready(SourceValue, SourceValue),
}

pub(super) struct Predictor {
    stage: Stage,
    // (first solution, second minus first). Later guesses always use the
    // original anchors so roundoff from advancing the sweep cannot accumulate.
    anchors: Vec<(f64, f64)>,
}
impl Predictor {
    pub fn new(n: usize) -> Self {
        Self {
            stage: Stage::Empty,
            anchors: vec![(0., 0.); n],
        }
    }
    pub fn seed(&mut self, next: SourceValue, x: &mut [f64]) {
        let (first, second) = match self.stage {
            Stage::Empty => {
                self.stage = Stage::First(next);
                return;
            }
            Stage::First(first) => {
                for (anchor, &v) in self.anchors.iter_mut().zip(x.iter()) {
                    anchor.0 = v;
                }
                self.stage = Stage::Second(first, next);
                return;
            }
            Stage::Second(first, second) => {
                for (anchor, &v) in self.anchors.iter_mut().zip(x.iter()) {
                    anchor.1 = v - anchor.0;
                }
                self.stage = Stage::Ready(first, second);
                (first, second)
            }
            Stage::Ready(first, second) => (first, second),
        };
        let fraction = match (first, second, next) {
            (SourceValue::Voltage(a), SourceValue::Voltage(b), SourceValue::Voltage(c)) => {
                ((c - a) / (b - a)).into_base()
            }
            (SourceValue::Current(a), SourceValue::Current(b), SourceValue::Current(c)) => {
                ((c - a) / (b - a)).into_base()
            }
            _ => unreachable!("sweep source dimension is fixed"),
        };
        if fraction.is_finite() {
            let mut finite = true;
            for (out, &(base, delta)) in x.iter_mut().zip(&self.anchors) {
                *out = delta.mul_add(fraction, base);
                finite &= out.is_finite();
            }
            if finite {
                return;
            }
        }
        x.fill(0.);
    }
}
