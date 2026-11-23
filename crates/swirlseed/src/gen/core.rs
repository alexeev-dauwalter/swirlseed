//! Shared building blocks of the generators. Mirrors `packages/swirlseed/src/gen/core.ts`
//! expression by expression: keep the two files in sync.

use crate::color::{linear_to_srgb255, oklab_to_linear};
pub use crate::dmath::{acos, asin, atan2, cbrt, cos, exp, ln, pow, sin, PI, TAU};
use crate::rng::{make_rng, Rng};
use std::sync::OnceLock;

pub const DEG: f64 = PI / 180.0;
pub type V3 = [f64; 3];

#[inline]
pub fn clamp(x: f64, a: f64, b: f64) -> f64 {
    if x < a {
        a
    } else if x > b {
        b
    } else {
        x
    }
}
#[inline]
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
#[inline]
pub fn sstep(a: f64, b: f64, x: f64) -> f64 {
    let t = clamp((x - a) / (b - a), 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
/// `Math.min` (NaN-propagating).
#[inline]
pub fn jmin(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a < b {
        a
    } else {
        b
    }
}
/// `Math.max` (NaN-propagating).
#[inline]
pub fn jmax(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a > b {
        a
    } else {
        b
    }
}
#[inline]
pub fn hypot2(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}
#[inline]
pub fn hypot3(x: f64, y: f64, z: f64) -> f64 {
    (x * x + y * y + z * z).sqrt()
}
/// Round half up, like `Math.round`.
#[inline]
pub fn round(x: f64) -> f64 {
    (x + 0.5).floor()
}
/// `x | 0` in JavaScript (ToInt32).
#[inline]
pub fn ti(x: f64) -> i32 {
    (x as i64) as i32
}
/// Float (0..255) to a byte, rounding half up.
#[inline]
pub fn to_byte(v: f64) -> u8 {
    if !(v > 0.0) {
        return 0;
    }
    if v >= 255.0 {
        return 255;
    }
    (v + 0.5).floor() as u8
}

/// A random stream in [0, 1) for one aspect of a generator.
pub struct Rand(Rng);
impl Rand {
    pub fn new(seed: &str, salt: &str) -> Rand {
        Rand(make_rng(seed, &format!("gen/{salt}")))
    }
    #[inline]
    pub fn r(&mut self) -> f64 {
        self.0.next_f64()
    }
}

#[inline]
pub fn hash2(x: f64, y: f64, s: f64) -> f64 {
    let h = ti(x)
        .wrapping_mul(374761393)
        .wrapping_add(ti(y).wrapping_mul(668265263))
        .wrapping_add(ti(s).wrapping_mul(1274126177)) as u32;
    let mut h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    h as f64 / 4294967296.0
}
#[inline]
pub fn hash3(x: f64, y: f64, z: f64, s: f64) -> f64 {
    let a = ti(x)
        .wrapping_mul(374761393)
        .wrapping_add(ti(y).wrapping_mul(668265263));
    let b = ti(z)
        .wrapping_mul(1440662683)
        .wrapping_add(ti(s).wrapping_mul(1274126177));
    let h = a.wrapping_add(b) as u32;
    let mut h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    h as f64 / 4294967296.0
}

pub fn vnoise(x: f64, y: f64, s: f64) -> f64 {
    let xi = x.floor();
    let yi = y.floor();
    let xf = x - xi;
    let yf = y - yi;
    let u = xf * xf * (3.0 - 2.0 * xf);
    let v = yf * yf * (3.0 - 2.0 * yf);
    let a = hash2(xi, yi, s);
    let b = hash2(xi + 1.0, yi, s);
    let c = hash2(xi, yi + 1.0, s);
    let d = hash2(xi + 1.0, yi + 1.0, s);
    a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v
}
pub fn fbm2(mut x: f64, mut y: f64, s: f64, o: u32) -> f64 {
    let mut t = 0.0;
    let mut a = 0.5;
    let mut n = 0.0;
    for i in 0..o {
        t += a * vnoise(x, y, s + (i * 31) as f64);
        n += a;
        a *= 0.5;
        let nx = (x * 0.8 + y * 0.6) * 2.03 + 11.7;
        let ny = (-x * 0.6 + y * 0.8) * 2.03 + 5.3;
        x = nx;
        y = ny;
    }
    t / n
}
pub fn fbm_s(mut x: f64, mut y: f64, s: f64, o: u32) -> f64 {
    let mut t = 0.0;
    let mut a = 0.5;
    let mut n = 0.0;
    for i in 0..o {
        t += a * vnoise(x, y, s + (i * 31) as f64);
        n += a;
        a *= 0.5;
        x *= 2.03;
        y *= 2.03;
    }
    t / n
}
pub fn vnoise3(x: f64, y: f64, z: f64, s: f64) -> f64 {
    let xi = x.floor();
    let yi = y.floor();
    let zi = z.floor();
    let xf = x - xi;
    let yf = y - yi;
    let zf = z - zi;
    let u = xf * xf * (3.0 - 2.0 * xf);
    let v = yf * yf * (3.0 - 2.0 * yf);
    let w = zf * zf * (3.0 - 2.0 * zf);
    let a = hash3(xi, yi, zi, s);
    let b = hash3(xi + 1.0, yi, zi, s);
    let c = hash3(xi, yi + 1.0, zi, s);
    let d = hash3(xi + 1.0, yi + 1.0, zi, s);
    let e = hash3(xi, yi, zi + 1.0, s);
    let f = hash3(xi + 1.0, yi, zi + 1.0, s);
    let g = hash3(xi, yi + 1.0, zi + 1.0, s);
    let h = hash3(xi + 1.0, yi + 1.0, zi + 1.0, s);
    let x1 = a + (b - a) * u;
    let x2 = c + (d - c) * u;
    let x3 = e + (f - e) * u;
    let x4 = g + (h - g) * u;
    let y1 = x1 + (x2 - x1) * v;
    let y2 = x3 + (x4 - x3) * v;
    y1 + (y2 - y1) * w
}
pub fn fbm3(mut x: f64, mut y: f64, mut z: f64, s: f64, o: u32) -> f64 {
    let mut t = 0.0;
    let mut a = 0.5;
    let mut n = 0.0;
    for i in 0..o {
        t += a * vnoise3(x, y, z, s + (i * 31) as f64);
        n += a;
        a *= 0.5;
        let nx = y * 2.03 + 17.1;
        let ny = z * 2.03 + 3.7;
        let nz = x * 2.03 + 9.3;
        x = nx;
        y = ny;
        z = nz;
    }
    t / n
}

// ---------- colour ----------
#[inline]
pub fn oklab2lin(l: f64, a: f64, b: f64) -> V3 {
    oklab_to_linear(l, a, b)
}
const ENC_N: usize = 4096;
fn enc_table() -> &'static Vec<f64> {
    static T: OnceLock<Vec<f64>> = OnceLock::new();
    T.get_or_init(|| {
        (0..=ENC_N)
            .map(|i| linear_to_srgb255(i as f64 / ENC_N as f64))
            .collect()
    })
}
/// Linear light (0..1) to sRGB 0..255 through a fine table (exact same table in TypeScript).
#[inline]
pub fn enc(v: f64) -> f64 {
    if !(v > 0.0) {
        return 0.0;
    }
    if v >= 1.0 {
        return 255.0;
    }
    let t = enc_table();
    let f = v * ENC_N as f64;
    let i = f.floor() as usize;
    t[i] + (t[i + 1] - t[i]) * (f - i as f64)
}
#[inline]
pub fn lin(c: f64) -> f64 {
    pow(c / 255.0, 2.2)
}
pub fn lin3(c: V3) -> V3 {
    [lin(c[0]), lin(c[1]), lin(c[2])]
}
pub fn hex_rgb(h: &str) -> V3 {
    let v = u32::from_str_radix(&h[1..], 16).unwrap_or(0);
    [((v >> 16) & 255) as f64, ((v >> 8) & 255) as f64, (v & 255) as f64]
}
pub fn srgb_to_lab(c: V3) -> V3 {
    let l = [lin(c[0]), lin(c[1]), lin(c[2])];
    let l_ = cbrt(0.4122214708 * l[0] + 0.5363325363 * l[1] + 0.0514459929 * l[2]);
    let m_ = cbrt(0.2119034982 * l[0] + 0.6806995451 * l[1] + 0.1073969566 * l[2]);
    let s_ = cbrt(0.0883024619 * l[0] + 0.2817188376 * l[1] + 0.6299787005 * l[2]);
    [
        0.2104542553 * l_ + 0.793617785 * m_ - 0.0040720468 * s_,
        1.9779984951 * l_ - 2.428592205 * m_ + 0.4505937099 * s_,
        0.0259040371 * l_ + 0.7827717662 * m_ - 0.808675766 * s_,
    ]
}
/// OKLCH -> OKLab, shrinking chroma until the colour fits (hue in degrees).
pub fn oklch_fit(l: f64, c0: f64, h: f64) -> V3 {
    let hr = h * DEG;
    let ch = cos(hr);
    let sh = sin(hr);
    let mut c = c0;
    for _ in 0..14 {
        let q = oklab2lin(l, c * ch, c * sh);
        if q[0] >= -0.001 && q[0] <= 1.001 && q[1] >= -0.001 && q[1] <= 1.001 && q[2] >= -0.001 && q[2] <= 1.001 {
            break;
        }
        c *= 0.88;
    }
    [l, c * ch, c * sh]
}

// ---------- images ----------
pub fn new_image(w: usize, h: usize) -> Vec<u8> {
    vec![0u8; w * h * 4]
}
#[inline]
pub fn put(d: &mut [u8], i: usize, r: f64, g: f64, b: f64) {
    let o = i * 4;
    d[o] = to_byte(r);
    d[o + 1] = to_byte(g);
    d[o + 2] = to_byte(b);
    d[o + 3] = 255;
}

/// Separable box blur on doubles, clamped at the edges.
pub fn blur(a: &[f64], w: usize, h: usize, r: i64) -> Vec<f64> {
    if r < 1 {
        return a.to_vec();
    }
    let (wi, hi) = (w as i64, h as i64);
    let mut t = vec![0.0f64; w * h];
    let mut o = vec![0.0f64; w * h];
    let n = (2 * r + 1) as f64;
    for y in 0..h {
        let row = y * w;
        let mut s = 0.0;
        for x in -r..=r {
            s += a[row + x.clamp(0, wi - 1) as usize];
        }
        for x in 0..wi {
            t[row + x as usize] = s / n;
            s += a[row + (x + r + 1).min(wi - 1) as usize] - a[row + (x - r).max(0) as usize];
        }
    }
    for x in 0..w {
        let mut s = 0.0;
        for y in -r..=r {
            s += t[y.clamp(0, hi - 1) as usize * w + x];
        }
        for y in 0..hi {
            o[y as usize * w + x] = s / n;
            s += t[(y + r + 1).min(hi - 1) as usize * w + x] - t[(y - r).max(0) as usize * w + x];
        }
    }
    o
}

// ---------- paper and pigment ----------
pub fn paper_height(w: usize, h: usize, s: f64, scale: f64) -> Vec<f64> {
    let mut out = vec![0.0f64; w * h];
    let m = (w.min(h)) as f64 / scale;
    for y in 0..h {
        for x in 0..w {
            let u = x as f64 / m;
            let v = y as f64 / m;
            out[y * w + x] = 0.5 * fbm2(u * 26.0, v * 26.0, s + 101.0, 3)
                + 0.3 * fbm2(u * 70.0, v * 70.0, s + 102.0, 2)
                + 0.2 * hash2(x as f64, y as f64, s + 103.0);
        }
    }
    out
}
pub fn paper_light(h: &[f64], w: usize, hh: usize, k: f64) -> Vec<f64> {
    let mut l = vec![0.0f64; w * hh];
    for y in 0..hh {
        for x in 0..w {
            let i = y * w + x;
            let xm = if x > 0 { h[i - 1] } else { h[i] };
            let ym = if y > 0 { h[i - w] } else { h[i] };
            l[i] = 1.0 + ((h[i] - xm) * 0.7 + (h[i] - ym) * 0.7) * k;
        }
    }
    l
}
pub struct PigmentOpts<'a> {
    pub s: f64,
    pub ph: Option<&'a [f64]>,
    pub gran: f64,
    pub edge: f64,
    pub edge_r: f64,
    pub mottle_f: f64,
    pub bloom: f64,
    pub bloom_f: f64,
    pub bloom_t: f64,
    pub strength: f64,
}
pub fn pigment_layer(ab: &mut [f64], a: &[f64], w: usize, h: usize, pig: V3, o: &PigmentOpts) {
    let m = w.min(h) as f64;
    let s = o.s;
    let k = [
        -ln(clamp(pig[0], 0.02, 0.999)),
        -ln(clamp(pig[1], 0.02, 0.999)),
        -ln(clamp(pig[2], 0.02, 0.999)),
    ];
    let er = jmax(1.0, round(m * o.edge_r)) as i64;
    let ab1 = blur(&blur(a, w, h, er), w, h, er);
    let fq = o.mottle_f;
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let av = a[i];
            if av < 0.003 {
                continue;
            }
            let u = x as f64 / m;
            let v = y as f64 / m;
            let edge = jmax(0.0, av - ab1[i]);
            let mot = 0.8 + 0.4 * fbm2(u * fq + s * 0.001, v * fq, s + 11.0, 4);
            let mut d = av * mot + edge * o.edge;
            if o.bloom > 0.0 {
                let n = fbm2(u * o.bloom_f, v * o.bloom_f, s + 21.0, 5);
                let t = o.bloom_t;
                let b = sstep(t, t + 0.015, n);
                d *= 1.0 - 0.3 * b * av;
                let q = (n - t) / 0.007;
                d += av * 0.55 * o.bloom * exp(-q * q);
            }
            let g = match o.ph {
                Some(ph) => 1.0 - o.gran + o.gran * 2.0 * ph[i] * (0.6 + 0.4 * fbm2(u * 40.0, v * 40.0, s + 31.0, 2)),
                None => 1.0,
            };
            d *= g * o.strength;
            ab[i * 3] += d * k[0];
            ab[i * 3 + 1] += d * k[1];
            ab[i * 3 + 2] += d * k[2];
        }
    }
}
pub fn develop_paper(ab: &[f64], w: usize, h: usize, paper: V3, pl: &[f64]) -> Vec<u8> {
    let mut d = new_image(w, h);
    for i in 0..w * h {
        let l = pl[i];
        put(
            &mut d,
            i,
            enc(paper[0] * exp(-ab[i * 3]) * l),
            enc(paper[1] * exp(-ab[i * 3 + 1]) * l),
            enc(paper[2] * exp(-ab[i * 3 + 2]) * l),
        );
    }
    d
}
