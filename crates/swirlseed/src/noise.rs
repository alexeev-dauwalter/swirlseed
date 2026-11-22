//! Noise, hashing for grain, Bayer matrix and box blur. Mirrors `noise.ts`.

use crate::dmath::{cos, sin, TAU};

/// Classic 2D gradient noise (16 gradient directions, quintic fade), range about [-1, 1].
pub struct Noise<'a> {
    perm: &'a [u8],
    gx: [f64; 16],
    gy: [f64; 16],
}

impl<'a> Noise<'a> {
    pub fn new(perm: &'a [u8]) -> Self {
        let mut gx = [0.0; 16];
        let mut gy = [0.0; 16];
        for i in 0..16 {
            gx[i] = cos((i as f64 * TAU) / 16.0);
            gy[i] = sin((i as f64 * TAU) / 16.0);
        }
        Noise { perm, gx, gy }
    }

    pub fn noise(&self, x: f64, y: f64) -> f64 {
        let perm = self.perm;
        let xi = x.floor();
        let yi = y.floor();
        let xf = x - xi;
        let yf = y - yi;
        let xx = ((xi as i64) & 255) as usize;
        let yy = ((yi as i64) & 255) as usize;
        let u = xf * xf * xf * (xf * (xf * 6.0 - 15.0) + 10.0);
        let v = yf * yf * yf * (yf * (yf * 6.0 - 15.0) + 10.0);
        let pa = perm[xx] as usize;
        let pb = perm[xx + 1] as usize;
        let g00 = (perm[pa + yy] & 15) as usize;
        let g01 = (perm[pa + yy + 1] & 15) as usize;
        let g10 = (perm[pb + yy] & 15) as usize;
        let g11 = (perm[pb + yy + 1] & 15) as usize;
        let n00 = self.gx[g00] * xf + self.gy[g00] * yf;
        let n10 = self.gx[g10] * (xf - 1.0) + self.gy[g10] * yf;
        let n01 = self.gx[g01] * xf + self.gy[g01] * (yf - 1.0);
        let n11 = self.gx[g11] * (xf - 1.0) + self.gy[g11] * (yf - 1.0);
        let a = n00 + u * (n10 - n00);
        let b = n01 + u * (n11 - n01);
        (a + v * (b - a)) * 1.41
    }

    /// Fractal Brownian motion with a rotation between octaves (hides the lattice).
    pub fn fbm(&self, mut x: f64, mut y: f64, oct: u32) -> f64 {
        let mut sum = 0.0;
        let mut amp = 0.5;
        let mut norm = 0.0;
        for _ in 0..oct {
            sum += amp * self.noise(x, y);
            norm += amp;
            let nx = (0.8 * x - 0.6 * y) * 2.03 + 17.3;
            y = (0.6 * x + 0.8 * y) * 2.03 + 9.1;
            x = nx;
            amp *= 0.5;
        }
        sum / norm
    }
}

/// Integer hash noise in [0, 1) for grain and dithering. x, y, k are small integers.
pub fn hash01(x: i32, y: i32, k: i32) -> f64 {
    let mut h = (x as u32)
        .wrapping_mul(374761393)
        .wrapping_add((y as u32).wrapping_mul(668265263))
        .wrapping_add((k as u32).wrapping_mul(2246822519));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    (h ^ (h >> 16)) as f64 / 4294967296.0
}

/// Bayer 8x8 ordered-dither thresholds in (-0.5, 0.5), stored as f32 like the JS `Float32Array`.
pub fn bayer8() -> [f32; 64] {
    let mut m: Vec<Vec<u32>> = vec![vec![0]];
    while m.len() < 8 {
        let n = m.len();
        let mut next = vec![vec![0u32; n * 2]; n * 2];
        for y in 0..n {
            for x in 0..n {
                let v = m[y][x] * 4;
                next[y][x] = v;
                next[y][x + n] = v + 2;
                next[y + n][x] = v + 3;
                next[y + n][x + n] = v + 1;
            }
        }
        m = next;
    }
    let mut out = [0.0f32; 64];
    for y in 0..8 {
        for x in 0..8 {
            out[y * 8 + x] = ((m[y][x] as f64 + 0.5) / 64.0 - 0.5) as f32;
        }
    }
    out
}

/// Separable box blur with edge clamping (running sum, O(N)).
pub fn box_blur(src: &[f32], w: usize, h: usize, r: usize) -> Vec<f32> {
    let mut tmp = vec![0.0f32; src.len()];
    let mut out = vec![0.0f32; src.len()];
    let d = (2 * r + 1) as f64;
    let ri = r as isize;
    for y in 0..h {
        let row = y * w;
        let mut sum = 0.0f64;
        for k in -ri..=ri {
            sum += src[row + k.clamp(0, w as isize - 1) as usize] as f64;
        }
        for x in 0..w {
            tmp[row + x] = (sum / d) as f32;
            sum += src[row + (w - 1).min(x + r + 1)] as f64 - src[row + x.saturating_sub(r)] as f64;
        }
    }
    for x in 0..w {
        let mut sum = 0.0f64;
        for k in -ri..=ri {
            sum += tmp[k.clamp(0, h as isize - 1) as usize * w + x] as f64;
        }
        for y in 0..h {
            out[y * w + x] = (sum / d) as f32;
            sum += tmp[(h - 1).min(y + r + 1) * w + x] as f64 - tmp[y.saturating_sub(r) * w + x] as f64;
        }
    }
    out
}
