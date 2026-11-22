//! Halftone pass. Mirrors `halftone.ts`.

// 0.5236 (30 degrees) and 0.7071 are deliberate decimal constants shared with the TypeScript code.
#![allow(clippy::approx_constant)]

use crate::color::lab_to_srgb255;
use crate::dmath::{cos, round_half_up, sin};
use crate::options::{HalftoneMode, HalftoneOptions, HalftoneShape};
use crate::params::Stop;

#[inline]
fn clamp01(v: f64) -> f64 {
    if v < 0.0 {
        0.0
    } else if v > 1.0 {
        1.0
    } else {
        v
    }
}

/// (r, g, b, luminance) of the pixel nearest to the image-space point (ix, iy).
fn sample_at(src: &[f32], w: usize, h: usize, ix: f64, iy: f64) -> (f64, f64, f64, f64) {
    let sx = ((w - 1) as f64).min(0.0f64.max(round_half_up(ix - 0.5))) as usize;
    let sy = ((h - 1) as f64).min(0.0f64.max(round_half_up(iy - 0.5))) as usize;
    let o3 = (sy * w + sx) * 3;
    let r = src[o3] as f64;
    let g = src[o3 + 1] as f64;
    let b = src[o3 + 2] as f64;
    let lum = clamp01((0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0);
    (r, g, b, lum)
}

/// Halftone pass over a float RGB buffer (0..255 per channel). `scale` = min(W, H) / 512.
pub fn apply_halftone(rgb: &mut [f32], w: usize, h: usize, scale: f64, o: &HalftoneOptions, stops: &[Stop]) {
    let cs = 2.0f64.max(o.size.unwrap_or(10.0) * scale);
    let ang = o.angle.unwrap_or(0.5236);
    let c = cos(ang);
    let s = sin(ang);
    let shape = o.shape.unwrap_or(HalftoneShape::Dot);
    let mono = o.mode.unwrap_or(HalftoneMode::Color) == HalftoneMode::Mono;
    let invert = o.invert.unwrap_or(false);
    let mix = clamp01(o.mix.unwrap_or(1.0));

    // "paper" and "ink" come from the lightest and darkest palette stops
    let mut sorted: Vec<Stop> = stops.to_vec();
    sorted.sort_by(|a, b| a.lab.l.partial_cmp(&b.lab.l).unwrap());
    let dark_c = lab_to_srgb255(&sorted[0].lab);
    let light_c = lab_to_srgb255(&sorted[sorted.len() - 1].lab);
    let paper = if invert { dark_c } else { light_c };
    let ink_mono = if invert { light_c } else { dark_c };

    let src: Vec<f32> = rgb.to_vec();

    for y in 0..h {
        for x in 0..w {
            let px = x as f64 + 0.5;
            let py = y as f64 + 0.5;
            let gx = px * c + py * s;
            let gy = -px * s + py * c;

            let mut best = 0.0f64;
            let (mut ir, mut ig, mut ib) = (0.0f64, 0.0f64, 0.0f64);

            if shape == HalftoneShape::Line {
                let j0 = (gy / cs - 0.5).floor();
                for dj in 0..2 {
                    let cyc = (j0 + dj as f64 + 0.5) * cs;
                    let (sr, sg, sb, slum) = sample_at(&src, w, h, gx * c - cyc * s, gx * s + cyc * c);
                    let a = if invert { slum } else { 1.0 - slum };
                    let half = 0.5 * cs * a;
                    let cov = clamp01(half - (gy - cyc).abs() + 0.5);
                    if cov > best {
                        best = cov;
                        ir = sr;
                        ig = sg;
                        ib = sb;
                    }
                }
            } else {
                let i0 = (gx / cs - 0.5).floor();
                let j0 = (gy / cs - 0.5).floor();
                for dj in 0..2 {
                    for di in 0..2 {
                        let cxc = (i0 + di as f64 + 0.5) * cs;
                        let cyc = (j0 + dj as f64 + 0.5) * cs;
                        let (sr, sg, sb, slum) = sample_at(&src, w, h, cxc * c - cyc * s, cxc * s + cyc * c);
                        let a = if invert { slum } else { 1.0 - slum };
                        let sq = a.sqrt();
                        let cov = if shape == HalftoneShape::Square {
                            let d = (gx - cxc).abs().max((gy - cyc).abs());
                            clamp01(0.5 * cs * sq - d + 0.5)
                        } else {
                            let ddx = gx - cxc;
                            let ddy = gy - cyc;
                            let d = (ddx * ddx + ddy * ddy).sqrt();
                            clamp01(0.7071 * cs * sq - d + 0.5)
                        };
                        if cov > best {
                            best = cov;
                            ir = sr;
                            ig = sg;
                            ib = sb;
                        }
                    }
                }
            }

            let (ink_r, ink_g, ink_b) = if mono {
                (ink_mono[0], ink_mono[1], ink_mono[2])
            } else {
                (ir, ig, ib)
            };
            let o3 = (y * w + x) * 3;
            let out_r = paper[0] + (ink_r - paper[0]) * best;
            let out_g = paper[1] + (ink_g - paper[1]) * best;
            let out_b = paper[2] + (ink_b - paper[2]) * best;
            let s0 = src[o3] as f64;
            let s1 = src[o3 + 1] as f64;
            let s2 = src[o3 + 2] as f64;
            rgb[o3] = (s0 + (out_r - s0) * mix) as f32;
            rgb[o3 + 1] = (s1 + (out_g - s1) * mix) as f32;
            rgb[o3 + 2] = (s2 + (out_b - s2) * mix) as f32;
        }
    }
}
