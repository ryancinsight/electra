//! SplitMix64 for repeatable engineering simulations.
pub(super) struct Uniform(pub(super) u64);
impl Uniform {
    pub(super) fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        ((z ^ (z >> 31)) >> 11) as f64 / 9007199254740992.
    }
}
