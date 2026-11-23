//! Flow field: evenly spaced streamlines (Jobard–Lefer) following a noise angle field, as tapered
//! strokes. Mirrors `flow.ts`.

use super::core::*;
use super::raster::{capsule, fill_polygon, paint, Mask};
use super::spec::*;

/// HSL (h in degrees, s and l in 0…1) to sRGB 0…255.
pub fn hsl(h: f64, s: f64, l: f64) -> V3 {
    let h = (((h % 360.0) + 360.0) % 360.0) / 360.0;
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let f = |mut t: f64| -> f64 {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            return p + (q - p) * 6.0 * t;
        }
        if t < 1.0 / 2.0 {
            return q;
        }
        if t < 2.0 / 3.0 {
            return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
        }
        p
    };
    [f(h + 1.0 / 3.0) * 255.0, f(h) * 255.0, f(h - 1.0 / 3.0) * 255.0]
}

fn bg(style: &str) -> &'static str {
    match style {
        "ink" => "#f3efe6",
        "neon" => "#06070e",
        "gold" => "#0d1a35",
        _ => "#1c0e2a",
    }
}

fn render(seed: &str, w: usize, h: usize, p: &Params) -> Vec<u8> {
    let mut rr = Rand::new(seed, "flow");
    let s_ = (rr.r() * 1e6).floor();
    let ox = rr.r() * 100.0;
    let oy = rr.r() * 100.0;
    let tilt = rr.r() * TAU;
    let style = p.str("style");
    let b = hex_rgb(bg(style));
    let mut img = vec![0.0f64; w * h * 3];
    for i in 0..w * h {
        img[i * 3] = b[0];
        img[i * 3 + 1] = b[1];
        img[i * 3 + 2] = b[2];
    }
    let wf = w as f64;
    let hf = h as f64;
    let sc = p.num("swirl") / hf;
    let ang = |x: f64, y: f64| -> f64 { (fbm2(x * sc + ox, y * sc + oy, s_, 4) - 0.5) * TAU * 2.6 + tilt };
    let dsep = jmax(3.0, wf * p.num("spacing"));
    let cell = dsep / std::f64::consts::SQRT_2;
    let gw = (wf / cell).ceil() as i64 + 1;
    let gh = (hf / cell).ceil() as i64 + 1;
    let mut grid: Vec<Vec<f64>> = vec![Vec::new(); (gw * gh) as usize];
    let step = dsep * 0.3;
    let max_steps = 30 + round(p.num("length") * 320.0) as i64;
    let free = |grid: &Vec<Vec<f64>>, x: f64, y: f64, dt: f64| -> bool {
        let cx = (x / cell).floor() as i64;
        let cy = (y / cell).floor() as i64;
        let r = (dt / cell).ceil() as i64;
        let t2 = dt * dt;
        for j in cy - r..=cy + r {
            if j < 0 || j >= gh {
                continue;
            }
            for i in cx - r..=cx + r {
                if i < 0 || i >= gw {
                    continue;
                }
                let c = &grid[(j * gw + i) as usize];
                let mut k = 0;
                while k < c.len() {
                    let dx = c[k] - x;
                    let dy = c[k + 1] - y;
                    if dx * dx + dy * dy < t2 {
                        return false;
                    }
                    k += 2;
                }
            }
        }
        true
    };
    let trace = |grid: &Vec<Vec<f64>>, sx: f64, sy: f64, back: bool| -> Vec<f64> {
        let mut out = Vec::new();
        let mut x = sx;
        let mut y = sy;
        let flip = if back { PI } else { 0.0 };
        for _ in 0..max_steps {
            let a1 = ang(x, y) + flip;
            let mx = x + cos(a1) * step * 0.5;
            let my = y + sin(a1) * step * 0.5;
            let a2 = ang(mx, my) + flip;
            x += cos(a2) * step;
            y += sin(a2) * step;
            if x < 0.0 || y < 0.0 || x >= wf || y >= hf || !free(grid, x, y, dsep * 0.62) {
                break;
            }
            out.push(x);
            out.push(y);
        }
        out
    };
    let mut lines: Vec<Vec<f64>> = Vec::new();
    let tries = round(((wf * hf) / (dsep * dsep)) * 1.4) as i64;
    for _ in 0..tries {
        let x = rr.r() * wf;
        let y = rr.r() * hf;
        if !free(&grid, x, y, dsep) {
            continue;
        }
        let f = trace(&grid, x, y, false);
        let bk = trace(&grid, x, y, true);
        let mut pts: Vec<f64> = Vec::with_capacity(f.len() + bk.len() + 2);
        let mut i = bk.len() as i64 - 2;
        while i >= 0 {
            pts.push(bk[i as usize]);
            pts.push(bk[i as usize + 1]);
            i -= 2;
        }
        pts.push(x);
        pts.push(y);
        pts.extend_from_slice(&f);
        if pts.len() < 12 {
            continue;
        }
        let mut i = 0;
        while i < pts.len() {
            let gi = ((pts[i + 1] / cell).floor() as i64 * gw + (pts[i] / cell).floor() as i64) as usize;
            grid[gi].push(pts[i]);
            grid[gi].push(pts[i + 1]);
            i += 2;
        }
        lines.push(pts);
    }
    let wmax = dsep * 0.34 * p.num("weight");
    let mut m = Mask::new(w, h);
    let mut xs: Vec<f64> = Vec::new();
    let mut ys: Vec<f64> = Vec::new();
    for pts in &lines {
        let n = pts.len() / 2;
        xs.clear();
        ys.clear();
        let mut rx: Vec<f64> = Vec::with_capacity(n);
        let mut ry: Vec<f64> = Vec::with_capacity(n);
        for i in 0..n {
            let i0 = if i > 0 { i - 1 } else { 0 };
            let i1 = (n - 1).min(i + 1);
            let mut tx = pts[i1 * 2] - pts[i0 * 2];
            let mut ty = pts[i1 * 2 + 1] - pts[i0 * 2 + 1];
            let mut l = hypot2(tx, ty);
            if l == 0.0 {
                l = 1.0;
            }
            tx /= l;
            ty /= l;
            let t = if n > 1 { i as f64 / (n - 1) as f64 } else { 0.5 };
            let wd = wmax * jmax(0.12, pow(sin(PI * t), 0.55));
            xs.push(pts[i * 2] - ty * wd);
            ys.push(pts[i * 2 + 1] + tx * wd);
            rx.push(pts[i * 2] + ty * wd);
            ry.push(pts[i * 2 + 1] - tx * wd);
        }
        for i in (0..n).rev() {
            xs.push(rx[i]);
            ys.push(ry[i]);
        }
        let mid = n >> 1;
        let a = ang(pts[mid * 2], pts[mid * 2 + 1]);
        let my = pts[mid * 2 + 1] / hf;
        let mx = pts[mid * 2] / wf;
        let col: V3;
        let mut alpha = 1.0;
        if style == "ink" {
            col = [21.0, 21.0, 26.0];
            alpha = 0.78 + 0.2 * rr.r();
        } else if style == "neon" {
            let hh = 190.0 + 110.0 * sin(a * 0.9 + mx * 3.0);
            col = hsl(hh, 1.0, 0.64);
            for i in 0..n.saturating_sub(1) {
                capsule(
                    &mut m,
                    pts[i * 2],
                    pts[i * 2 + 1],
                    pts[i * 2 + 2],
                    pts[i * 2 + 3],
                    wmax * 3.4,
                );
            }
            let g = hsl(hh, 1.0, 0.6);
            paint(&mut m, &mut img, g[0], g[1], g[2], 0.16);
        } else if style == "gold" {
            let hh = 36.0 + 14.0 * sin(a) + 8.0 * rr.r();
            let s = (62.0 + 20.0 * rr.r()) / 100.0;
            col = hsl(hh, s, (52.0 + 16.0 * rr.r()) / 100.0);
        } else {
            let hh = lerp(345.0, 50.0, clamp(my + 0.18 * sin(a), 0.0, 1.0));
            col = hsl(hh, 0.92, (58.0 + 8.0 * rr.r()) / 100.0);
        }
        fill_polygon(&mut m, &xs, &ys, xs.len());
        paint(&mut m, &mut img, col[0], col[1], col[2], alpha);
    }
    let mut d = new_image(w, h);
    for i in 0..w * h {
        put(&mut d, i, img[i * 3], img[i * 3 + 1], img[i * 3 + 2]);
    }
    d
}

pub const FLOW: Generator = Generator {
    id: "flow-field",
    title: "Flow field",
    description: "Thousands of evenly spaced, tapered streamlines that follow a noise field, like plotter art.",
    aspect: 1.5,
    params: &[
        choice(
            "style",
            "Style",
            "ink",
            &[
                opt("ink", "Ink"),
                opt("neon", "Neon"),
                opt("gold", "Gold"),
                opt("sunset", "Sunset"),
            ],
        ),
        number("spacing", "Spacing", 0.005, 0.022, 0.0005, 0.009, ""),
        number("swirl", "Swirl", 0.5, 4.0, 0.05, 1.6, "x"),
        number("weight", "Weight", 0.4, 2.6, 0.05, 1.2, "x"),
        number("length", "Length", 0.1, 1.0, 0.02, 0.7, "%"),
    ],
    render,
};
