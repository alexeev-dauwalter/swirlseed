//! Deterministic rasteriser. Mirrors `raster.ts`.

use super::core::{clamp, jmax, jmin};

const SUB: f64 = 4.0;

pub struct Mask {
    pub w: usize,
    pub h: usize,
    pub cov: Vec<f64>,
    pub x0: i64,
    pub y0: i64,
    pub x1: i64,
    pub y1: i64,
}

impl Mask {
    pub fn new(w: usize, h: usize) -> Mask {
        Mask {
            w,
            h,
            cov: vec![0.0; w * h],
            x0: 0,
            y0: 0,
            x1: -1,
            y1: -1,
        }
    }
    pub fn reset_box(&mut self) {
        self.x0 = 0;
        self.y0 = 0;
        self.x1 = -1;
        self.y1 = -1;
    }
    pub fn grow(&mut self, x0: i64, y0: i64, x1: i64, y1: i64) {
        if self.x1 < self.x0 {
            self.x0 = x0;
            self.y0 = y0;
            self.x1 = x1;
            self.y1 = y1;
            return;
        }
        self.x0 = self.x0.min(x0);
        self.y0 = self.y0.min(y0);
        self.x1 = self.x1.max(x1);
        self.y1 = self.y1.max(y1);
    }
    /// Iterate the pixels of the bounding box (caller handles coverage).
    pub fn bbox(&self) -> (i64, i64, i64, i64) {
        (self.x0, self.y0, self.x1, self.y1)
    }
}

/// Fill a closed polygon (non-zero winding) into the mask, adding coverage.
pub fn fill_polygon(m: &mut Mask, xs: &[f64], ys: &[f64], n: usize) {
    let w = m.w as i64;
    let h = m.h as i64;
    let mut ymin = f64::INFINITY;
    let mut ymax = f64::NEG_INFINITY;
    for i in 0..n {
        if ys[i] < ymin {
            ymin = ys[i];
        }
        if ys[i] > ymax {
            ymax = ys[i];
        }
    }
    let k0 = jmax(0.0, (ymin * SUB - 0.5).ceil());
    let k1 = jmin((h as f64) * SUB - 1.0, (ymax * SUB - 0.5).ceil() - 1.0);
    if k1 < k0 {
        return;
    }
    let (k0, k1) = (k0 as i64, k1 as i64);
    let mut rows: Vec<Vec<f64>> = (k0..=k1).map(|_| Vec::new()).collect();
    for i in 0..n {
        let j = if i + 1 < n { i + 1 } else { 0 };
        let (xa, ya, xb, yb) = (xs[i], ys[i], xs[j], ys[j]);
        if ya == yb {
            continue;
        }
        let dir = if yb > ya { 1.0 } else { -1.0 };
        let lo = jmin(ya, yb);
        let hi = jmax(ya, yb);
        let ka = jmax(k0 as f64, (lo * SUB - 0.5).ceil()) as i64;
        let kb = jmin(k1 as f64, (hi * SUB - 0.5).ceil() - 1.0) as i64;
        let sl = (xb - xa) / (yb - ya);
        let mut k = ka;
        while k <= kb {
            let sy = (k as f64 + 0.5) / SUB;
            let r = &mut rows[(k - k0) as usize];
            r.push(xa + (sy - ya) * sl);
            r.push(dir);
            k += 1;
        }
    }
    let mut bx0 = w;
    let mut bx1: i64 = -1;
    let mut idx: Vec<usize> = Vec::new();
    for k in k0..=k1 {
        let r = &rows[(k - k0) as usize];
        let c = r.len() / 2;
        if c < 2 {
            continue;
        }
        idx.clear();
        idx.extend(0..c);
        idx.sort_by(|&p, &q| {
            let a = r[p * 2];
            let b = r[q * 2];
            a.partial_cmp(&b)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    r[p * 2 + 1]
                        .partial_cmp(&r[q * 2 + 1])
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
                .then(p.cmp(&q))
        });
        let py = k.div_euclid(SUB as i64);
        let row = (py * w) as usize;
        let mut wind = 0.0;
        for q in 0..c - 1 {
            wind += r[idx[q] * 2 + 1];
            if wind == 0.0 {
                continue;
            }
            let xa = clamp(r[idx[q] * 2], 0.0, w as f64);
            let xb = clamp(r[idx[q + 1] * 2], 0.0, w as f64);
            if xb <= xa {
                continue;
            }
            let ia = xa.floor() as i64;
            let ib = (w - 1).min(xb.ceil() as i64 - 1);
            let mut x = ia;
            while x <= ib {
                let ov = jmin(xb, (x + 1) as f64) - jmax(xa, x as f64);
                if ov > 0.0 {
                    m.cov[row + x as usize] += ov / SUB;
                }
                x += 1;
            }
            if ia < bx0 {
                bx0 = ia;
            }
            if ib > bx1 {
                bx1 = ib;
            }
        }
    }
    if bx1 >= bx0 {
        m.grow(bx0, k0.div_euclid(SUB as i64), bx1, k1.div_euclid(SUB as i64));
    }
}

/// Round-capped segment of width w; coverage is max-combined.
pub fn capsule(m: &mut Mask, ax: f64, ay: f64, bx: f64, by: f64, w: f64) {
    let r = w / 2.0;
    let x0 = jmax(0.0, (jmin(ax, bx) - r - 1.0).floor()) as i64;
    let x1 = jmin(m.w as f64 - 1.0, (jmax(ax, bx) + r + 1.0).ceil()) as i64;
    let y0 = jmax(0.0, (jmin(ay, by) - r - 1.0).floor()) as i64;
    let y1 = jmin(m.h as f64 - 1.0, (jmax(ay, by) + r + 1.0).ceil()) as i64;
    if x1 < x0 || y1 < y0 {
        return;
    }
    let dx = bx - ax;
    let dy = by - ay;
    let ll = dx * dx + dy * dy;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let px = x as f64 + 0.5 - ax;
            let py = y as f64 + 0.5 - ay;
            let t = if ll > 0.0 {
                clamp((px * dx + py * dy) / ll, 0.0, 1.0)
            } else {
                0.0
            };
            let ex = px - dx * t;
            let ey = py - dy * t;
            let c = clamp(r - (ex * ex + ey * ey).sqrt() + 0.5, 0.0, 1.0) * (if w < 1.0 { w } else { 1.0 });
            let i = y as usize * m.w + x as usize;
            if c > m.cov[i] {
                m.cov[i] = c;
            }
        }
    }
    m.grow(x0, y0, x1, y1);
}

/// Filled disc; coverage is max-combined.
pub fn disc(m: &mut Mask, cx: f64, cy: f64, r: f64) {
    let x0 = jmax(0.0, (cx - r - 1.0).floor()) as i64;
    let x1 = jmin(m.w as f64 - 1.0, (cx + r + 1.0).ceil()) as i64;
    let y0 = jmax(0.0, (cy - r - 1.0).floor()) as i64;
    let y1 = jmin(m.h as f64 - 1.0, (cy + r + 1.0).ceil()) as i64;
    if x1 < x0 || y1 < y0 {
        return;
    }
    for y in y0..=y1 {
        for x in x0..=x1 {
            let ex = x as f64 + 0.5 - cx;
            let ey = y as f64 + 0.5 - cy;
            let c = clamp(r - (ex * ex + ey * ey).sqrt() + 0.5, 0.0, 1.0);
            let i = y as usize * m.w + x as usize;
            if c > m.cov[i] {
                m.cov[i] = c;
            }
        }
    }
    m.grow(x0, y0, x1, y1);
}

/// Paint a solid colour through the mask into an RGB float buffer. Clears the mask.
pub fn paint(m: &mut Mask, img: &mut [f64], r: f64, g: f64, b: f64, alpha: f64) {
    let (x0, y0, x1, y1) = m.bbox();
    for y in y0..=y1 {
        for x in x0..=x1 {
            let i = y as usize * m.w + x as usize;
            let mut c = m.cov[i];
            if c <= 0.0 {
                continue;
            }
            if c > 1.0 {
                c = 1.0;
            }
            let a = c * alpha;
            img[i * 3] += (r - img[i * 3]) * a;
            img[i * 3 + 1] += (g - img[i * 3 + 1]) * a;
            img[i * 3 + 2] += (b - img[i * 3 + 2]) * a;
            m.cov[i] = 0.0;
        }
    }
    m.reset_box();
}
