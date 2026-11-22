//! Rendering. Mirrors `render.ts` expression by expression; buffers are `f32` where the
//! TypeScript code uses `Float32Array`, and every `f32` is widened to `f64` before arithmetic.

use crate::color::lab_to_srgb255;
use crate::dmath::{cos, exp, pow, round_half_up, sin, TAU};
use crate::halftone::apply_halftone;
use crate::noise::{bayer8, box_blur, hash01, Noise};
use crate::options::RenderOptions;
use crate::params::{create_pattern, Mode, Params, Stop};

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

/// Same conversion as JavaScript's `Uint8ClampedArray`: clamp, then round half to even.
#[inline]
fn to_u8_clamped(v: f64) -> u8 {
    if v.is_nan() || v <= 0.0 {
        0
    } else if v >= 255.0 {
        255
    } else {
        v.round_ties_even() as u8
    }
}

fn build_lut(stops: &[Stop], size: usize) -> Vec<f32> {
    let mut s: Vec<Stop> = stops.to_vec();
    s.sort_by(|p, q| p.pos.partial_cmp(&q.pos).unwrap());
    let mut lut = vec![0.0f32; size * 3];
    for i in 0..size {
        let t = i as f64 / (size - 1) as f64;
        let lab = if t <= s[0].pos {
            s[0].lab
        } else if t >= s[s.len() - 1].pos {
            s[s.len() - 1].lab
        } else {
            let mut k = 0;
            while t > s[k + 1].pos {
                k += 1;
            }
            let p0 = &s[k];
            let p1 = &s[k + 1];
            let d = p1.pos - p0.pos;
            let mut f = (t - p0.pos) / (if d == 0.0 || d.is_nan() { 1.0 } else { d });
            f = f * f * (3.0 - 2.0 * f); // smoothstep between neighbouring stops
            crate::color::Lab {
                l: p0.lab.l + (p1.lab.l - p0.lab.l) * f,
                a: p0.lab.a + (p1.lab.a - p0.lab.a) * f,
                b: p0.lab.b + (p1.lab.b - p0.lab.b) * f,
            }
        };
        let rgb = lab_to_srgb255(&lab);
        lut[i * 3] = rgb[0] as f32;
        lut[i * 3 + 1] = rgb[1] as f32;
        lut[i * 3 + 2] = rgb[2] as f32;
    }
    lut
}

/// Render a pattern to RGBA (`width * height * 4` bytes).
/// The base field is smooth, so 512x512 is plenty; textures are computed in real pixels.
pub fn render_pattern(p: &Params, width: u32, height: u32, o: &RenderOptions) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let n = w * h;
    let wf = width as f64;
    let hf = height as f64;
    let noise = Noise::new(&p.perm);

    let min_side = width.min(height) as f64;
    let scale = min_side / 512.0; // texture scale: same look at any resolution
    let aspect = wf / hf;
    let ca = cos(p.angle);
    let sa = sin(p.angle);
    let inv_stretch = 1.0 / p.stretch;
    let (cx, cy, strength, radius) = (p.swirl.cx, p.swirl.cy, p.swirl.strength, p.swirl.radius);
    let rad2 = radius * radius;

    // Reeded glass setup
    let has_ribs = o.ribs.is_some();
    let (mut rc, mut rs, mut rw, mut r_str, mut r_shade) = (1.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
    if let Some(r) = &o.ribs {
        let a = r.angle.unwrap_or(0.0);
        rc = cos(a);
        rs = sin(a);
        rw = 1.0 / 1.0f64.max(r.count.unwrap_or(24.0));
        r_str = r.strength.unwrap_or(0.6);
        r_shade = r.shade.unwrap_or(0.5);
    }
    let mut rib_fr = vec![0.0f32; if has_ribs { n } else { 0 }];

    // Pass 1: scalar field
    let mut field = vec![0.0f32; n];
    for y in 0..h {
        for x in 0..w {
            let mut xx = ((x as f64 + 0.5) / wf - 0.5) * aspect;
            let mut yy = (y as f64 + 0.5) / hf - 0.5;

            if has_ribs {
                let s_ = xx * rc + yy * rs;
                let k = s_ / rw;
                let fr = k - k.floor() - 0.5; // -0.5..0.5 inside a flute
                let d = -fr * rw * 2.0 * r_str;
                xx += d * rc;
                yy += d * rs;
                rib_fr[y * w + x] = fr as f32;
            }

            let dx = xx - cx;
            let dy = yy - cy;
            let ang = strength * exp(-(dx * dx + dy * dy) / rad2);
            let c = cos(ang);
            let s = sin(ang);
            xx = cx + dx * c - dy * s;
            yy = cy + dx * s + dy * c;

            let mut u = (xx * ca + yy * sa) * p.base_freq * inv_stretch;
            let mut v = (-xx * sa + yy * ca) * p.base_freq;

            for l in &p.layers {
                let wx = noise.fbm(u * l.freq + l.ox, v * l.freq + l.oy, 3);
                let wy = noise.fbm(u * l.freq + l.ox2, v * l.freq + l.oy2, 3);
                u += l.amp * wx;
                v += l.amp * wy;
            }

            field[y * w + x] = (v + p.fbm_weight * noise.fbm(u * 0.8 + 31.7, v * 0.8 - 12.3, 3)) as f32;
        }
    }

    // Normalise by the 2nd-98th percentile, sampled on a 2D grid at the same normalised points
    // for every frame size, so the look does not depend on the resolution.
    let g = 96usize;
    let mut sample: Vec<f64> = Vec::with_capacity(g * g);
    for gy in 0..g {
        let sy = (((gy as f64 + 0.5) * hf) / g as f64).floor() as usize;
        for gx in 0..g {
            let sx = (((gx as f64 + 0.5) * wf) / g as f64).floor() as usize;
            sample.push(field[sy * w + sx] as f64);
        }
    }
    sample.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let lo = sample[(sample.len() as f64 * 0.02).floor() as usize];
    let hi = sample[(sample.len() as f64 * 0.98).floor() as usize];
    let dd = hi - lo;
    let inv = 1.0 / (if dd == 0.0 || dd.is_nan() { 1.0 } else { dd });

    let mut tm = vec![0.0f32; n];
    for i in 0..n {
        let mut t = clamp01((field[i] as f64 - lo) * inv);
        if p.mode == Mode::Bands {
            t = 0.5 - 0.5 * cos(TAU * (t * p.repeats + p.phase));
        }
        tm[i] = pow(t, p.gamma) as f32;
    }

    // Pass 2: palette + lighting -> float RGB (0..255)
    const LUT_N: usize = 2048;
    let lut = build_lut(&p.stops, LUT_N);
    let mut rgb = vec![0.0f32; n * 3];

    let shine = o.shine.unwrap_or(p.shine);
    // Height map for lighting: a smoothed copy removes creases on plateaus and flute edges.
    let blurred;
    let hm: &[f32] = if shine > 0.0 {
        let br = 1.0f64.max(round_half_up(1.8 * scale)) as usize;
        blurred = box_blur(&box_blur(&tm, w, h, br), w, h, br);
        &blurred
    } else {
        &tm
    };
    // light from the top-left, plus the half vector for the specular highlight
    let llen = (-0.45f64 * -0.45 + -0.6 * -0.6 + 0.66 * 0.66).sqrt();
    let lx = -0.45 / llen;
    let ly = -0.6 / llen;
    let lz = 0.66 / llen;
    let hlen = (lx * lx + ly * ly + (lz + 1.0) * (lz + 1.0)).sqrt();
    let hx = lx / hlen;
    let hy = ly / hlen;
    let hz = (lz + 1.0) / hlen;
    let mut flat_spec = hz; // hz^64
    for _ in 0..6 {
        flat_spec *= flat_spec;
    }
    let slope_k = 0.35 * min_side; // derivative per frame unit, not per pixel

    for y in 0..h {
        let ym = if y > 0 { y - 1 } else { y };
        let yp = if y < h - 1 { y + 1 } else { y };
        for x in 0..w {
            let i = y * w + x;
            let li = round_half_up(tm[i] as f64 * (LUT_N - 1) as f64) as usize * 3;
            let mut r = lut[li] as f64;
            let mut g = lut[li + 1] as f64;
            let mut b = lut[li + 2] as f64;

            if shine > 0.0 {
                let xm = if x > 0 { x - 1 } else { x };
                let xp = if x < w - 1 { x + 1 } else { x };
                let dxs = if xp == xm { 1.0 } else { (xp - xm) as f64 };
                let dys = if yp == ym { 1.0 } else { (yp - ym) as f64 };
                let gx = ((hm[y * w + xp] as f64 - hm[y * w + xm] as f64) / dxs) * slope_k;
                let gy = ((hm[yp * w + x] as f64 - hm[ym * w + x] as f64) / dys) * slope_k;
                let il = 1.0 / (gx * gx + gy * gy + 1.0).sqrt();
                let nx = -gx * il;
                let ny = -gy * il;
                let nz = il;
                let diff = nx * lx + ny * ly + nz * lz - lz; // 0 on a flat surface
                let mut sp = clamp01(nx * hx + ny * hy + nz * hz);
                for _ in 0..6 {
                    sp *= sp; // ^64
                }
                let spec = 0.0f64.max(sp - flat_spec);
                let k = 0.45f64.max(1.6f64.min(1.0 + shine * diff * 1.4));
                let add = shine * spec * 230.0;
                r = r * k + add;
                g = g * k + add;
                b = b * k + add;
            }

            if has_ribs {
                let k = 1.0 + r_shade * rib_fr[i] as f64 * 0.6;
                r *= k;
                g *= k;
                b *= k;
            }

            let o3 = i * 3;
            rgb[o3] = r as f32;
            rgb[o3 + 1] = g as f32;
            rgb[o3 + 2] = b as f32;
        }
    }

    // Pass 3: halftone
    if let Some(ht) = &o.halftone {
        apply_halftone(&mut rgb, w, h, scale, ht, &p.stops);
    }

    // Pass 4: grain
    if let Some(gr) = &o.grain {
        let amount = gr.amount.unwrap_or(0.35) * 70.0;
        let size = 1.0f64.max(round_half_up(gr.size.unwrap_or(1.0) * scale)) as i32;
        for y in 0..h {
            for x in 0..w {
                let nn = hash01(x as i32 / size, y as i32 / size, 7) - 0.5;
                let o3 = (y * w + x) * 3;
                rgb[o3] = (rgb[o3] as f64 + nn * amount) as f32;
                rgb[o3 + 1] = (rgb[o3 + 1] as f64 + nn * amount) as f32;
                rgb[o3 + 2] = (rgb[o3 + 2] as f64 + nn * amount) as f32;
            }
        }
    }

    // Pass 5: quantise (Bayer dithering, or a faint dither that hides banding)
    let mut out = vec![0u8; n * 4];
    let dz = o.dither.as_ref();
    let levels = match dz {
        Some(d) => 32.0f64.min(2.0f64.max(round_half_up(d.levels.unwrap_or(4.0)))),
        None => 256.0,
    };
    let d_scale = 1.0f64.max(round_half_up(dz.and_then(|d| d.scale).unwrap_or(1.5) * scale)) as usize;
    let d_amount = dz.and_then(|d| d.amount).unwrap_or(1.0);
    let bayer = bayer8();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let o3 = i * 3;
            let o4 = i * 4;
            if dz.is_some() {
                let thr = 0.5 + d_amount * bayer[(((y / d_scale) & 7) << 3) | ((x / d_scale) & 7)] as f64;
                for c in 0..3 {
                    let v = clamp01(rgb[o3 + c] as f64 / 255.0) * (levels - 1.0);
                    out[o4 + c] = to_u8_clamped(((v + thr).floor() / (levels - 1.0)) * 255.0);
                }
            } else {
                let nn = hash01(x as i32, y as i32, 1) - 0.5;
                out[o4] = to_u8_clamped(rgb[o3] as f64 + nn);
                out[o4 + 1] = to_u8_clamped(rgb[o3 + 1] as f64 + nn);
                out[o4 + 2] = to_u8_clamped(rgb[o3 + 2] as f64 + nn);
            }
            out[o4 + 3] = 255;
        }
    }
    out
}

/// String -> RGBA.
pub fn generate(seed: &str, width: u32, height: u32, options: &RenderOptions) -> Vec<u8> {
    render_pattern(&create_pattern(seed), width, height, options)
}
