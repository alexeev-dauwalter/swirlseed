//! OKLab / OKLCH helpers. Mirrors `color.ts`.

use crate::dmath::{cos, pow, sin, PI};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

pub fn oklab_to_linear(l: f64, a: f64, b: f64) -> [f64; 3] {
    let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l - 0.0894841775 * a - 1.291485548 * b;
    let l3 = l_ * l_ * l_;
    let m3 = m_ * m_ * m_;
    let s3 = s_ * s_ * s_;
    [
        4.0767416621 * l3 - 3.3077115913 * m3 + 0.2309699292 * s3,
        -1.2684380046 * l3 + 2.6097574011 * m3 - 0.3413193965 * s3,
        -0.0041960863 * l3 - 0.7034186147 * m3 + 1.707614701 * s3,
    ]
}

pub fn linear_to_srgb255(v: f64) -> f64 {
    let v = if v < 0.0 {
        0.0
    } else if v > 1.0 {
        1.0
    } else {
        v
    };
    255.0
        * (if v <= 0.0031308 {
            12.92 * v
        } else {
            1.055 * pow(v, 1.0 / 2.4) - 0.055
        })
}

/// OKLCH -> OKLab, reducing chroma by bisection until the colour fits into sRGB.
pub fn oklch_fit(l: f64, c: f64, h_deg: f64) -> Lab {
    let h = (h_deg * PI) / 180.0;
    let ch = cos(h);
    let sh = sin(h);
    let in_gamut = |c: f64| -> bool {
        let [r, g, b] = oklab_to_linear(l, c * ch, c * sh);
        let e = 1e-4;
        r >= -e && r <= 1.0 + e && g >= -e && g <= 1.0 + e && b >= -e && b <= 1.0 + e
    };
    let mut c = c;
    if !in_gamut(c) {
        let mut lo = 0.0;
        let mut hi = c;
        for _ in 0..14 {
            let mid = (lo + hi) / 2.0;
            if in_gamut(mid) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        c = lo;
    }
    Lab {
        l,
        a: c * ch,
        b: c * sh,
    }
}

/// OKLab -> sRGB, each channel 0..255 (not rounded).
pub fn lab_to_srgb255(l: &Lab) -> [f64; 3] {
    let [r, g, b] = oklab_to_linear(l.l, l.a, l.b);
    [linear_to_srgb255(r), linear_to_srgb255(g), linear_to_srgb255(b)]
}
