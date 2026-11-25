//! One stamp traversal, specialized for matrix-vector products, rank checks and direct factors.
use super::{Host, Signal, Stamp, System};
use athena_core::LinearOperator;

pub(super) trait Equations {
    fn inject(&mut self, stamp: &Stamp, signal: Signal);
    fn voltage(&mut self, row: usize, signal: Signal, gain: f64);
}

impl System {
    pub(super) fn stamp(&self, equations: &mut impl Equations) {
        for r in &self.resistors {
            equations.inject(r, Signal::Difference(r.a, r.b));
        }
        let mut controls = self.controls.voltages.iter().peekable();
        for (j, s) in self.sources.iter().enumerate() {
            let slot = self.node_count() + j;
            equations.inject(&s.stamp, Signal::Unknown(slot));
            let drop = Signal::Difference(s.stamp.a, s.stamp.b);
            if let Some(v) = controls.next_if(|v| v.row == slot) {
                equations.voltage(slot, drop, v.output_weight);
                equations.voltage(slot, v.signal, -v.input_weight);
            } else {
                equations.voltage(slot, drop, 1.);
            }
        }
        for (stamp, signal) in &self.controls.currents {
            equations.inject(stamp, *signal);
        }
    }
}
struct Product<'a> {
    x: &'a [f64],
    out: &'a mut [f64],
}
impl Equations for Product<'_> {
    #[inline]
    fn inject(&mut self, stamp: &Stamp, signal: Signal) {
        let value = signal.read(self.x);
        self.out[stamp.a] += stamp.at_a * value;
        self.out[stamp.b] -= stamp.at_b * value;
    }
    #[inline]
    fn voltage(&mut self, row: usize, signal: Signal, gain: f64) {
        self.out[row] += gain * signal.read(self.x);
    }
}
impl LinearOperator<Host> for System {
    fn dimension(&self) -> usize {
        self.node_count() + self.sources.len()
    }
    fn apply(&self, _: &Host, x: &[f64], out: &mut [f64]) -> Result<(), &'static str> {
        if x.len() != self.dimension() || out.len() != x.len() {
            return Err("operator dimension mismatch");
        }
        out.fill(0.);
        self.stamp(&mut Product { x, out });
        // One pinned zero slot lets every edge use original node indices.
        out[0] = x[0];
        Ok(())
    }
}
