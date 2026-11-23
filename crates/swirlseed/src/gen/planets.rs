//! Planets and nebulae: per-pixel shaded spheres (3D noise terrain, clouds, rings) and
//! domain-warped nebulae. Mirrors `planets.ts`.

use super::core::*;
use super::raster::{capsule, Mask};
use super::spec::*;

fn mix3(a: V3, b: V3, t: f64) -> V3 {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}
fn ramp(s: &[V3], t: f64) -> V3 {
    let t = clamp(t, 0.0, 1.0) * (s.len() - 1) as f64;
    let i = jmin((s.len() - 2) as f64, t.floor()) as usize;
    mix3(s[i], s[i + 1], t - i as f64)
}
const GAS: [[[f64; 3]; 5]; 3] = [
    [
        [228.0, 196.0, 150.0],
        [190.0, 140.0, 96.0],
        [240.0, 222.0, 190.0],
        [158.0, 100.0, 70.0],
        [214.0, 170.0, 120.0],
    ],
    [
        [210.0, 226.0, 236.0],
        [120.0, 158.0, 190.0],
        [232.0, 240.0, 246.0],
        [86.0, 124.0, 164.0],
        [170.0, 196.0, 214.0],
    ],
    [
        [236.0, 190.0, 120.0],
        [176.0, 98.0, 60.0],
        [246.0, 226.0, 180.0],
        [120.0, 70.0, 52.0],
        [208.0, 150.0, 96.0],
    ],
];
fn c(r: f64, g: f64, b: f64) -> V3 {
    lin3([r, g, b])
}

fn render(seed: &str, w: usize, h: usize, p: &Params) -> Vec<u8> {
    let ocean_s = c(70.0, 150.0, 186.0);
    let ocean_d = c(8.0, 28.0, 78.0);
    let forest = c(34.0, 78.0, 34.0);
    let grass = c(88.0, 120.0, 52.0);
    let desert = c(196.0, 160.0, 104.0);
    let rock = c(120.0, 104.0, 92.0);
    let snow = c(240.0, 240.0, 244.0);
    let icecap = c(236.0, 242.0, 248.0);
    let ice_a = c(196.0, 226.0, 240.0);
    let ice_b = c(240.0, 248.0, 252.0);
    let ice_c = c(92.0, 148.0, 190.0);
    let spot_a = c(190.0, 70.0, 50.0);
    let spot_b = c(232.0, 170.0, 120.0);
    let ring_a = c(150.0, 128.0, 104.0);
    let ring_b = c(226.0, 206.0, 170.0);

    let mut rr = Rand::new(seed, "planets");
    let s_ = (rr.r() * 1e6).floor();
    let ty = p.str("type");
    let neb = ty == "nebula";
    let ringed = ty == "ringed-giant";
    let giant = ty == "gas-giant" || ringed;
    let clouds = p.num("clouds");
    let (wf, hf) = (w as f64, h as f64);
    let m = jmin(wf, hf);
    let mut sb = vec![0.0f64; w * h];
    let ns = round(((if neb { 1.1 } else { 0.6 }) * wf * hf) / 1500.0) as i64;
    let mut bright: Vec<f64> = Vec::new();
    for _ in 0..ns {
        let x = (rr.r() * wf).floor() as usize;
        let y = (rr.r() * hf).floor() as usize;
        let b = pow(rr.r(), 3.4);
        let hue = rr.r();
        sb[y * w + x] = jmax(sb[y * w + x], 0.2 + b * 0.8);
        if b > 0.72 && neb {
            bright.extend_from_slice(&[x as f64, y as f64, b, hue]);
        }
    }
    let sun_a = p.num("sun") * DEG;
    let mut lx = sin(sun_a) * 0.92;
    let mut ly = -0.28;
    let mut lz = cos(sun_a) * 0.92;
    {
        let l = hypot3(lx, ly, lz);
        lx /= l;
        ly /= l;
        lz /= l;
    }
    let r = (if ringed { 0.72 } else { 1.0 }) * p.num("size") * hf;
    let cx = wf / 2.0;
    let cy = hf / 2.0;
    let inv = 1.0 / r;
    let tilt = if ringed { 0.32 + rr.r() * 0.3 } else { 0.0 };
    let roll = (rr.r() - 0.5) * 0.5;
    let mut npx = 0.0;
    let mut npy = cos(tilt);
    let npz = -sin(tilt);
    {
        let cc = cos(roll);
        let ss = sin(roll);
        let x = npx * cc - npy * ss;
        npy = npx * ss + npy * cc;
        npx = x;
    }
    let gi = (rr.r() * 3.0).floor() as usize;
    let gas: Vec<V3> = GAS[gi].iter().map(|c| lin3(*c)).collect();
    let fq = 2.6 + rr.r() * 2.6;
    let sx = 0.3 + rr.r() * 0.2;
    let sy0 = rr.r() * 0.3;
    let sy = 0.18 + sy0 * (if rr.r() < 0.5 { 1.0 } else { -1.0 });
    rr.r();
    let ox = rr.r() * 50.0;
    let oy = rr.r() * 50.0;
    let oz = rr.r() * 50.0;
    let wl = 0.38 + p.num("water") * 0.24;
    let h_a = rr.r() * 360.0;
    let h_b = h_a + 60.0 + rr.r() * 90.0;
    let h_c = h_a + 180.0 + (rr.r() - 0.5) * 60.0;
    let c_a = oklch_fit(0.5, 0.17, h_a);
    let c_b = oklch_fit(0.62, 0.2, h_b % 360.0);
    let c_c = oklch_fit(0.42, 0.13, h_c % 360.0);
    let l_a = oklab2lin(c_a[0], c_a[1], c_a[2]);
    let l_b = oklab2lin(c_b[0], c_b[1], c_b[2]);
    let l_c = oklab2lin(c_c[0], c_c[1], c_c[2]);
    let ring_at = |rho: f64| -> f64 {
        if rho < 1.28 || rho > 2.35 {
            return 0.0;
        }
        let t = (rho - 1.28) / 1.07;
        let mut o = 0.35 + 0.6 * vnoise(rho * 70.0, 0.0, s_) * (0.5 + vnoise(rho * 13.0, 1.0, s_ + 2.0));
        o *= sstep(0.0, 0.05, t) * sstep(1.0, 0.93, t);
        o *= 1.0 - 0.92 * sstep(0.045, 0.0, (rho - 1.87).abs());
        o *= 1.0 - 0.8 * sstep(0.03, 0.0, (rho - 2.02).abs());
        clamp(o, 0.0, 0.97)
    };
    let ring_col = |rho: f64| -> V3 {
        mix3(
            ring_a,
            ring_b,
            clamp(
                0.5 + 0.5 * sin(rho * 37.0) + 0.25 * vnoise(rho * 50.0, 3.0, s_),
                0.0,
                1.0,
            ),
        )
    };
    let hvx = lx;
    let hvy = ly;
    let hvz = lz + 1.0;
    let hl = hypot3(hvx, hvy, hvz);
    let mut out = vec![0.0f64; w * h * 3];
    for y in 0..h {
        for x in 0..w {
            let (xf, yf) = (x as f64, y as f64);
            let u = (xf - cx) * inv;
            let v = (yf - cy) * inv;
            let mut r_;
            let mut g_;
            let mut b_;
            if neb {
                let px = (xf / m) * 2.1;
                let py = (yf / m) * 2.1;
                let q = fbm2(px + ox, py + oy, s_, 4);
                let wv = fbm2(px + 4.0 * q + 1.7, py + 4.0 * q + 9.2, s_ + 7.0, 4);
                let f = fbm2(px + 3.2 * wv + 8.0, py + 3.2 * wv + 2.0, s_ + 13.0, 5);
                let dens = pow(clamp(f * 1.45 - 0.18, 0.0, 1.0), 1.9);
                let t2 = clamp(wv * 1.5 - 0.2, 0.0, 1.0);
                let col = mix3(mix3(l_a, l_b, t2), l_c, clamp((q - 0.45) * 3.0, 0.0, 1.0) * 0.7);
                let lane = 1.0
                    - 0.88
                        * sstep(0.5, 0.72, fbm2(px * 2.6 + 30.0, py * 2.6 + 11.0, s_ + 21.0, 4))
                        * sstep(0.1, 0.5, f);
                let cq = clamp(f * 1.9 - 0.9, 0.0, 1.0);
                let core = cq * cq * 1.5;
                r_ = (col[0] * dens * 1.25 + core * 0.3) * lane + 0.004;
                g_ = (col[1] * dens * 1.25 + core * 0.27) * lane + 0.005;
                b_ = (col[2] * dens * 1.25 + core * 0.25) * lane + 0.01;
            } else {
                let t = (yf / hf) * (yf / hf);
                let nb = fbm2((xf / m) * 1.6 + ox, (yf / m) * 1.6 + oy, s_ + 3.0, 3);
                r_ = 0.003 + 0.004 * t + 0.012 * nb * nb * (l_a[0] + 0.2);
                g_ = 0.004 + 0.006 * t + 0.012 * nb * nb * (l_a[1] + 0.2);
                b_ = 0.01 + 0.016 * t + 0.02 * nb * nb * (l_a[2] + 0.2);
            }
            let st1 = sb[y * w + x];
            if st1 > 0.0 {
                r_ += st1;
                g_ += st1 * 0.97;
                b_ += st1 * (if neb { 1.0 } else { 0.9 });
            }
            let d2 = u * u + v * v;
            let rn = d2.sqrt();
            if !neb {
                if rn < 1.0 + 2.0 / r {
                    let nz = jmax(0.0, 1.0 - d2).sqrt();
                    let nx = u;
                    let ny = v;
                    let cov = clamp((1.0 - rn) * r + 0.5, 0.0, 1.0);
                    let ndl = nx * lx + ny * ly + nz * lz;
                    let lit = sstep(-0.12, 0.35, ndl) * 0.9 + 0.1 * clamp(ndl + 0.2, 0.0, 1.0);
                    let mut col: V3;
                    let mut spec = 0.0;
                    let mut cl = 0.0;
                    let xx = nx + ox;
                    let yy = ny + oy;
                    let zz = nz + oz;
                    if ty == "rocky" {
                        let wq = fbm3(xx * 1.2 + 7.0, yy * 1.2, zz * 1.2, s_ + 40.0, 3) * 0.9;
                        let hgt = fbm3(xx * 1.9 + wq, yy * 1.9, zz * 1.9 + wq, s_, 7) * 0.82
                            + 0.18 * fbm3(xx * 9.0, yy * 9.0, zz * 9.0, s_ + 9.0, 4);
                        let lat = ny.abs();
                        if hgt < wl {
                            col = mix3(ocean_s, ocean_d, pow(clamp((wl - hgt) / 0.28, 0.0, 1.0), 0.6));
                            let hd = clamp((nx * hvx + ny * hvy + nz * hvz) / hl, 0.0, 1.0);
                            spec = pow(hd, 260.0) * 0.7 + pow(hd, 24.0) * 0.06;
                        } else {
                            let k = clamp((hgt - wl) / 0.3, 0.0, 1.0);
                            let dry = clamp(
                                fbm3(xx * 2.4 + 5.0, yy * 2.4, zz * 2.4, s_ + 4.0, 3) * 1.5 - 0.3 + lat * 0.2,
                                0.0,
                                1.0,
                            );
                            col = mix3(
                                mix3(
                                    mix3(forest, grass, clamp(dry * 2.0, 0.0, 1.0)),
                                    desert,
                                    clamp(dry * 2.0 - 1.0, 0.0, 1.0),
                                ),
                                rock,
                                k * k,
                            );
                            if k > 0.8 {
                                col = mix3(col, snow, (k - 0.8) * 4.0);
                            }
                        }
                        col = mix3(
                            col,
                            icecap,
                            sstep(
                                0.8,
                                0.9,
                                lat + (fbm3(xx * 4.0, yy * 4.0, zz * 4.0, s_ + 6.0, 3) - 0.5) * 0.14,
                            ),
                        );
                        let wx = fbm3(xx * 1.6 + 3.0, yy * 1.6, zz * 1.6, s_ + 11.0, 3) * 2.2;
                        let wy = fbm3(xx * 1.6, yy * 1.6 + 5.0, zz * 1.6, s_ + 13.0, 3) * 2.2;
                        let cn = fbm3(xx * 2.4 + wx, yy * 3.2 + wy, zz * 2.4 + wx, s_ + 12.0, 7);
                        let fine = fbm3(xx * 14.0, yy * 14.0, zz * 14.0, s_ + 14.0, 3);
                        cl = clamp(
                            sstep(0.63 - 0.14 * clouds, 0.74 - 0.12 * clouds, cn * (0.85 + 0.3 * fine))
                                * (0.75 + 0.25 * fine)
                                * jmin(1.0, clouds * 2.0),
                            0.0,
                            0.97,
                        );
                    } else if ty == "ice" {
                        let hgt = fbm3(xx * 2.2, yy * 2.2, zz * 2.2, s_, 5);
                        let cr = (fbm3(xx * 3.1 + 9.0, yy * 3.1, zz * 3.1, s_ + 5.0, 4) - 0.5).abs();
                        col = mix3(
                            mix3(ice_a, ice_b, hgt),
                            ice_c,
                            (1.0 - sstep(0.0, 0.035, cr)) * 0.8 + sstep(0.5, 0.0, hgt) * 0.15,
                        );
                        spec = pow(clamp(nz, 0.0, 1.0), 12.0) * 0.12;
                        cl = clouds
                            * 0.3
                            * clamp(fbm3(xx * 3.0, yy * 3.0, zz * 3.0, s_ + 12.0, 4) * 2.0 - 0.7, 0.0, 1.0);
                    } else {
                        let wv = fbm3(xx * 2.2, yy * 2.2, zz * 2.2, s_ + 3.0, 4);
                        let lat = ny * fq;
                        let shear = sin(lat * 3.1) * 1.3;
                        let ed = fbm3(xx * 5.0 + shear, yy * 14.0, zz * 5.0 - shear, s_ + 15.0, 5);
                        let b = lat
                            + wv * 0.55
                            + (ed - 0.5) * 0.55 * cos(lat * 3.1).abs()
                            + fbm3(xx * 9.0, yy * 9.0 * 0.2, zz * 9.0, s_ + 8.0, 3) * 0.15;
                        col = ramp(
                            &gas,
                            0.5 + 0.42 * sin(b * 3.1) + 0.18 * sin(b * 7.3 + 1.0) + 0.25 * (wv - 0.5),
                        );
                        let ex = (nx - sx) / 0.34;
                        let ey = (ny - sy) / 0.17;
                        let e2 = ex * ex + ey * ey;
                        if e2 < 1.6 {
                            let sw = atan2(ey, ex) + e2 * 3.0;
                            col = mix3(
                                col,
                                mix3(spot_a, spot_b, 0.5 + 0.5 * sin(sw * 2.0)),
                                sstep(1.6, 0.4, e2) * 0.85,
                            );
                        }
                    }
                    let mut shade = 1.0;
                    if ringed {
                        let dn = npx * lx + npy * ly + npz * lz;
                        if dn.abs() > 1e-4 {
                            let t = -(npx * nx + npy * ny + npz * nz) / dn;
                            if t > 0.0 {
                                shade = 1.0 - ring_at(hypot3(nx + t * lx, ny + t * ly, nz + t * lz)) * 0.85;
                            }
                        }
                    }
                    let dl = lit * shade;
                    let mut pr = col[0] * dl;
                    let mut pg = col[1] * dl;
                    let mut pb = col[2] * dl;
                    if cl > 0.0 {
                        let cd = dl * 0.95 + 0.03;
                        pr = pr * (1.0 - cl) + cd * cl;
                        pg = pg * (1.0 - cl) + cd * cl;
                        pb = pb * (1.0 - cl) + cd * cl;
                    }
                    pr += spec * lit;
                    pg += spec * lit;
                    pb += spec * lit;
                    let rim = pow(1.0 - nz, 3.2) * clamp(ndl * 1.6 + 0.35, 0.0, 1.0);
                    let ac: V3 = if giant {
                        [0.5, 0.4, 0.3]
                    } else if ty == "ice" {
                        [0.35, 0.65, 1.0]
                    } else {
                        [0.22, 0.5, 1.0]
                    };
                    let am = if giant { 0.25 } else { 0.9 };
                    pr += ac[0] * rim * am;
                    pg += ac[1] * rim * am;
                    pb += ac[2] * rim * am;
                    r_ = r_ * (1.0 - cov) + pr * cov;
                    g_ = g_ * (1.0 - cov) + pg * cov;
                    b_ = b_ * (1.0 - cov) + pb * cov;
                }
                if rn > 1.0 {
                    let gl = exp(-(rn - 1.0) * 16.0)
                        * clamp((u * lx + v * ly) * 0.9 + 0.5, 0.0, 1.0)
                        * (if giant { 0.18 } else { 0.55 });
                    let ac: V3 = if ty == "ice" {
                        [0.3, 0.55, 1.0]
                    } else if ty == "rocky" {
                        [0.18, 0.42, 1.0]
                    } else {
                        [0.5, 0.4, 0.3]
                    };
                    r_ += ac[0] * gl;
                    g_ += ac[1] * gl;
                    b_ += ac[2] * gl;
                }
                if ringed {
                    let zz = -(npx * u + npy * v) / npz;
                    let rho = hypot3(u, v, zz);
                    let o = ring_at(rho);
                    if o > 0.0 && (rn >= 1.0 || zz > 0.0) {
                        let mut lt = 0.9;
                        let bb = u * lx + v * ly + zz * lz;
                        if bb < 0.0 && rho * rho - bb * bb < 1.0 {
                            lt = 0.08;
                        }
                        let rc = ring_col(rho);
                        r_ = r_ * (1.0 - o) + rc[0] * o * lt;
                        g_ = g_ * (1.0 - o) + rc[1] * o * lt;
                        b_ = b_ * (1.0 - o) + rc[2] * o * lt;
                    }
                }
            }
            let i = (y * w + x) * 3;
            out[i] = enc(1.0 - exp(-r_));
            out[i + 1] = enc(1.0 - exp(-g_));
            out[i + 2] = enc(1.0 - exp(-b_));
        }
    }
    if neb {
        let mut mask = Mask::new(w, h);
        let mut k = 0;
        while k < bright.len() {
            let sx2 = bright[k];
            let sy2 = bright[k + 1];
            let b = bright[k + 2];
            let hh = bright[k + 3];
            let rr2 = m * (0.008 + 0.02 * b);
            let col: V3 = if hh < 0.33 {
                [255.0, 225.0, 190.0]
            } else if hh < 0.66 {
                [200.0, 220.0, 255.0]
            } else {
                [255.0, 255.0, 255.0]
            };
            let x0 = jmax(0.0, (sx2 - rr2).floor()) as i64;
            let x1 = jmin(wf - 1.0, (sx2 + rr2).ceil()) as i64;
            let y0 = jmax(0.0, (sy2 - rr2).floor()) as i64;
            let y1 = jmin(hf - 1.0, (sy2 + rr2).ceil()) as i64;
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let dx = x as f64 + 0.5 - sx2;
                    let dy = y as f64 + 0.5 - sy2;
                    let t = (dx * dx + dy * dy).sqrt() / rr2;
                    if t >= 1.0 {
                        continue;
                    }
                    let a = if t < 0.25 {
                        0.8 * b + (0.25 * b - 0.8 * b) * (t / 0.25)
                    } else {
                        0.25 * b * (1.0 - (t - 0.25) / 0.75)
                    };
                    let i = (y as usize * w + x as usize) * 3;
                    out[i] += col[0] * a;
                    out[i + 1] += col[1] * a;
                    out[i + 2] += col[2] * a;
                }
            }
            if b > 0.85 {
                let l = rr2 * 2.2;
                let lw = jmax(1.0, m * 0.0012);
                capsule(&mut mask, sx2 - l, sy2, sx2 + l, sy2, lw);
                capsule(&mut mask, sx2, sy2 - l, sx2, sy2 + l, lw);
                for y in mask.y0..=mask.y1 {
                    for x in mask.x0..=mask.x1 {
                        let i = y as usize * w + x as usize;
                        let c = jmin(1.0, mask.cov[i]) * 0.5 * b;
                        out[i * 3] += col[0] * c;
                        out[i * 3 + 1] += col[1] * c;
                        out[i * 3 + 2] += col[2] * c;
                        mask.cov[i] = 0.0;
                    }
                }
                mask.reset_box();
            }
            k += 4;
        }
    }
    let mut d = new_image(w, h);
    for i in 0..w * h {
        put(&mut d, i, out[i * 3], out[i * 3 + 1], out[i * 3 + 2]);
    }
    d
}

pub const PLANETS: Generator = Generator {
    id: "planets",
    title: "Planets & nebulae",
    description:
        "Worlds lit by a distant sun: oceans and clouds, cracked ice, banded and ringed giants, and glowing nebulae.",
    aspect: 1.5,
    params: &[
        choice(
            "type",
            "Type",
            "rocky",
            &[
                opt("rocky", "Rocky"),
                opt("ice", "Ice"),
                opt("gas-giant", "Gas giant"),
                opt("ringed-giant", "Ringed giant"),
                opt("nebula", "Nebula"),
            ],
        ),
        number("sun", "Sun", -170.0, 170.0, 1.0, -55.0, "°"),
        number("size", "Size", 0.25, 0.5, 0.01, 0.38, "%"),
        number("clouds", "Clouds", 0.0, 1.0, 0.01, 0.55, "%"),
        number("water", "Water", 0.0, 1.0, 0.01, 0.5, "%"),
    ],
    render,
};
