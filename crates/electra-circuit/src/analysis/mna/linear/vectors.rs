use athena_core::KrylovBackend;

pub(in crate::analysis::mna) struct Host;

fn length(a: usize, b: usize) -> Result<(), &'static str> {
    if a == b {
        Ok(())
    } else {
        Err("vector length mismatch")
    }
}

// Pairwise reduction bounds rounding growth with vector length. Short leaves
// keep the inner multiply/add loop contiguous for the compiler.
fn inner(a: &[f64], b: &[f64]) -> f64 {
    if a.len() <= 32 {
        return a.iter().zip(b).map(|(x, y)| x * y).sum();
    }
    let mid = a.len() / 2;
    let (al, ar) = a.split_at(mid);
    let (bl, br) = b.split_at(mid);
    inner(al, bl) + inner(ar, br)
}

impl KrylovBackend for Host {
    type Scalar = f64;
    type Error = &'static str;
    type Vector = Vec<f64>;
    type VectorBlock = (Vec<f64>, usize);
    type View<'a> = &'a [f64];
    type ViewMut<'a> = &'a mut [f64];
    type PreparedNorm = ();
    type PreparedDot = ();

    fn allocate(&self, len: usize) -> Result<Vec<f64>, Self::Error> {
        let mut v = Vec::new();
        v.try_reserve_exact(len)
            .map_err(|_| "Krylov workspace allocation failed")?;
        v.resize(len, 0.);
        Ok(v)
    }
    fn allocate_block(&self, count: usize, len: usize) -> Result<Self::VectorBlock, Self::Error> {
        Ok((
            self.allocate(count.checked_mul(len).ok_or("basis too large")?)?,
            len,
        ))
    }
    fn view<'a>(&'a self, v: &'a Vec<f64>) -> &'a [f64] {
        v
    }
    fn view_mut<'a>(&'a self, v: &'a mut Vec<f64>) -> &'a mut [f64] {
        v
    }
    fn block_view<'a>(&'a self, b: &'a Self::VectorBlock, i: usize) -> &'a [f64] {
        &b.0[i * b.1..(i + 1) * b.1]
    }
    fn block_view_mut<'a>(&'a self, b: &'a mut Self::VectorBlock, i: usize) -> &'a mut [f64] {
        &mut b.0[i * b.1..(i + 1) * b.1]
    }
    fn vector_len(&self, v: &Vec<f64>) -> usize {
        v.len()
    }
    fn copy(&self, s: &[f64], t: &mut [f64]) -> Result<(), Self::Error> {
        length(s.len(), t.len())?;
        t.copy_from_slice(s);
        Ok(())
    }
    fn scale(&self, t: &mut [f64], a: f64) -> Result<(), Self::Error> {
        for x in t {
            *x *= a;
        }
        Ok(())
    }
    fn axpy(&self, t: &mut [f64], s: &[f64], a: f64) -> Result<(), Self::Error> {
        length(t.len(), s.len())?;
        for (x, &v) in t.iter_mut().zip(s) {
            *x += a * v;
        }
        Ok(())
    }
    fn prepare_dot(&self, a: &[f64], b: &[f64]) -> Result<(), Self::Error> {
        length(a.len(), b.len())
    }
    fn dot_prepared(&self, _: &(), a: &[f64], b: &[f64]) -> Result<f64, Self::Error> {
        length(a.len(), b.len())?;
        Ok(inner(a, b))
    }
    fn prepare_norm_l2(&self, _: &[f64]) -> Result<(), Self::Error> {
        Ok(())
    }
    fn norm_l2_prepared(&self, _: &(), v: &[f64]) -> Result<f64, Self::Error> {
        // Preserve NaNs and use scaled accumulation when direct squaring loses range.
        let sum = inner(v, v);
        if sum.is_normal() || sum.is_nan() {
            return Ok(sum.sqrt());
        }
        let scale = v.iter().fold(0.0_f64, |s, x| s.max(x.abs()));
        if scale == 0. || !scale.is_finite() {
            return Ok(scale);
        }
        Ok(scale * v.iter().map(|x| (x / scale).powi(2)).sum::<f64>().sqrt())
    }
    fn residual(&self, b: &[f64], a: &[f64], r: &mut [f64]) -> Result<(), Self::Error> {
        length(b.len(), a.len())?;
        length(a.len(), r.len())?;
        for ((r, &b), &a) in r.iter_mut().zip(b).zip(a) {
            *r = b - a;
        }
        Ok(())
    }
    fn fused_cg_update(
        &self,
        x: &mut [f64],
        p: &[f64],
        r: &mut [f64],
        ap: &[f64],
        alpha: f64,
    ) -> Result<(), Self::Error> {
        length(x.len(), r.len())?;
        length(r.len(), ap.len())?;
        self.axpy(x, p, alpha)?;
        self.axpy(r, ap, -alpha)
    }
    fn combine_direction(&self, p: &mut [f64], z: &[f64], beta: f64) -> Result<(), Self::Error> {
        length(p.len(), z.len())?;
        for (p, &z) in p.iter_mut().zip(z) {
            *p = z + beta * *p;
        }
        Ok(())
    }
}
