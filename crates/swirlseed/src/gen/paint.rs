//! Paint on paper: watercolour, wet-on-wet, paste paper and dendritic painting. Mirrors `paint.ts`.

use super::core::*;
use super::raster::{capsule, disc, fill_polygon, Mask};
use super::spec::*;

type Pal = Vec<(V3, f64)>;
fn mk_pal(a: &[([f64; 3], f64)]) -> Pal {
    a.iter().map(|(c, g)| (lin3(*c), *g)).collect()
}

// ---------------- watercolour ----------------
fn wc(id: &str) -> &'static [([f64; 3], f64)] {
    match id {
        "sunset" => &[
            ([220.0, 70.0, 50.0], 0.3),
            ([240.0, 150.0, 40.0], 0.2),
            ([200.0, 50.0, 110.0], 0.3),
            ([110.0, 60.0, 140.0], 0.8),
            ([250.0, 200.0, 80.0], 0.1),
        ],
        "forest" => &[
            ([50.0, 110.0, 60.0], 0.4),
            ([140.0, 150.0, 50.0], 0.3),
            ([30.0, 80.0, 90.0], 0.6),
            ([150.0, 100.0, 50.0], 0.8),
            ([90.0, 120.0, 40.0], 0.4),
        ],
        "rose" => &[
            ([210.0, 70.0, 110.0], 0.3),
            ([230.0, 120.0, 120.0], 0.2),
            ([140.0, 70.0, 150.0], 0.6),
            ([240.0, 170.0, 90.0], 0.2),
            ([90.0, 110.0, 170.0], 0.7),
        ],
        "sepia" => &[
            ([110.0, 70.0, 40.0], 0.8),
            ([150.0, 100.0, 60.0], 0.7),
            ([70.0, 50.0, 40.0], 0.9),
            ([180.0, 140.0, 90.0], 0.5),
            ([80.0, 90.0, 110.0], 0.8),
        ],
        _ => &[
            ([40.0, 80.0, 170.0], 0.8),
            ([20.0, 120.0, 150.0], 0.5),
            ([70.0, 60.0, 150.0], 0.9),
            ([200.0, 120.0, 60.0], 0.6),
            ([30.0, 90.0, 80.0], 0.3),
        ],
    }
}

fn gauss(r: &mut Rand) -> f64 {
    let mut u = 0.0;
    while u == 0.0 {
        u = r.r();
    }
    (-2.0 * ln(u)).sqrt() * cos(TAU * r.r())
}
fn deform(r: &mut Rand, p: &[f64], depth: usize, vr: f64) -> Vec<f64> {
    let mut p = p.to_vec();
    for _ in 0..depth {
        let n = p.len() / 2;
        let mut q = Vec::with_capacity(p.len() * 2);
        for i in 0..n {
            let j = (i + 1) % n;
            let ax = p[i * 2];
            let ay = p[i * 2 + 1];
            let bx = p[j * 2];
            let by = p[j * 2 + 1];
            let l = hypot2(bx - ax, by - ay);
            let gx = gauss(r);
            let gy = gauss(r);
            q.extend_from_slice(&[ax, ay, (ax + bx) / 2.0 + gx * vr * l, (ay + by) / 2.0 + gy * vr * l]);
        }
        p = q;
    }
    p
}
fn blob(r: &mut Rand, cx: f64, cy: f64, rx: f64, ry: f64, n: usize) -> Vec<f64> {
    let mut p = Vec::with_capacity(n * 2);
    let ph = r.r() * TAU;
    for i in 0..n {
        let a = ph + (i as f64 / n as f64) * TAU;
        let rr = 0.75 + 0.5 * r.r();
        p.push(cx + cos(a) * rx * rr);
        p.push(cy + sin(a) * ry * rr);
    }
    p
}
fn layer(m: &mut Mask, a: &mut [f64], p: &[f64], alpha: f64) {
    let n = p.len() / 2;
    let xs: Vec<f64> = (0..n).map(|i| p[i * 2]).collect();
    let ys: Vec<f64> = (0..n).map(|i| p[i * 2 + 1]).collect();
    fill_polygon(m, &xs, &ys, n);
    accumulate(m, a, alpha);
}
fn accumulate(m: &mut Mask, a: &mut [f64], alpha: f64) {
    let w = m.w as i64;
    for y in m.y0..=m.y1 {
        for x in m.x0..=m.x1 {
            let i = (y * w + x) as usize;
            let mut c = m.cov[i];
            if c <= 0.0 {
                continue;
            }
            if c > 1.0 {
                c = 1.0;
            }
            a[i] += c * alpha * (1.0 - a[i]);
            m.cov[i] = 0.0;
        }
    }
    m.reset_box();
}

fn render_watercolor(seed: &str, w: usize, h: usize, p: &Params) -> Vec<u8> {
    let mut rr = Rand::new(seed, "watercolor");
    let s_ = (rr.r() * 1e6).floor();
    let (wf, hf) = (w as f64, h as f64);
    let m = jmin(wf, hf);
    let pal = mk_pal(wc(p.str("palette")));
    let ph = paper_height(w, h, s_, 1.0);
    let pl = paper_light(&ph, w, h, 1.1);
    let mut ab = vec![0.0f64; w * h * 3];
    let mut mask = Mask::new(w, h);
    let n_w = round(p.num("washes")) as i64;
    let layers = round(p.num("layers")) as i64;
    let bleed = p.num("bleed");
    for wi in 0..n_w {
        let big = wi < 2;
        let cx = wf * (0.1 + 0.8 * rr.r());
        let cy = hf * (0.1 + 0.8 * rr.r());
        let r = m * (if big { 0.3 + 0.15 * rr.r() } else { 0.1 + 0.17 * rr.r() });
        let (pig, gr) = pal[(rr.r() * pal.len() as f64).floor() as usize];
        let mut a = vec![0.0f64; w * h];
        let rx = r * (1.0 + 0.5 * rr.r());
        let ry = r * (0.7 + 0.4 * rr.r());
        let b0 = blob(&mut rr, cx, cy, rx, ry, 10);
        let base = deform(&mut rr, &b0, 3, 0.22);
        let la = 1.6 / layers as f64;
        for _ in 0..layers {
            let vr = 0.1 + 0.06 * rr.r();
            let poly = deform(&mut rr, &base, 3, vr);
            layer(&mut mask, &mut a, &poly, la);
        }
        let ns = round(p.num("splatter") * 40.0 * (if big { 0.5 } else { 1.0 })) as i64;
        for _ in 0..ns {
            let an = rr.r() * TAU;
            let dd = r * (0.9 + rr.r() * 1.3);
            let q = rr.r();
            let sr = m * 0.0022 * (0.4 + q * q * 4.0);
            disc(&mut mask, cx + cos(an) * dd, cy + sin(an) * dd * 0.8, sr);
            accumulate(&mut mask, &mut a, 0.42);
        }
        for v in a.iter_mut() {
            *v = jmin(1.0, *v * 1.05);
        }
        let edge = 2.2 + 1.6 * rr.r();
        let mottle_f = 2.0 + 3.0 * rr.r();
        let bloom = if rr.r() < bleed * 0.8 { 0.8 } else { 0.0 };
        let bloom_f = 4.0 + 4.0 * rr.r();
        let bloom_t = 0.56 + 0.06 * rr.r();
        let strength = if big { 0.55 + 0.4 * rr.r() } else { 0.8 + 0.6 * rr.r() };
        pigment_layer(
            &mut ab,
            &a,
            w,
            h,
            pig,
            &PigmentOpts {
                s: s_ + (wi * 97) as f64,
                ph: Some(&ph),
                gran: gr * 0.8,
                edge,
                edge_r: 0.005,
                mottle_f,
                bloom,
                bloom_f,
                bloom_t,
                strength,
            },
        );
        if rr.r() < bleed {
            let mut b = vec![0.0f64; w * h];
            let bx = cx + (rr.r() - 0.5) * r * 1.6;
            let by = cy + (rr.r() - 0.5) * r * 1.6;
            for _ in 0..10 {
                let b1 = blob(&mut rr, bx, by, r * 0.5, r * 0.45, 8);
                let poly = deform(&mut rr, &b1, 4, 0.2);
                layer(&mut mask, &mut b, &poly, 0.5);
            }
            let br = round(m * 0.02) as i64;
            let bm = blur(&blur(&b, w, h, br), w, h, br);
            let (p2, g2) = pal[(rr.r() * pal.len() as f64).floor() as usize];
            pigment_layer(
                &mut ab,
                &bm,
                w,
                h,
                p2,
                &PigmentOpts {
                    s: s_ + (wi * 31) as f64 + 7.0,
                    ph: Some(&ph),
                    gran: g2 * 0.8,
                    edge: 0.6,
                    edge_r: 0.006,
                    mottle_f: 3.0,
                    bloom: 0.0,
                    bloom_f: 5.0,
                    bloom_t: 0.58,
                    strength: 0.6,
                },
            );
        }
    }
    develop_paper(&ab, w, h, lin3([247.0, 243.0, 233.0]), &pl)
}

pub const WATERCOLOR: Generator = Generator {
    id: "watercolor",
    title: "Watercolor",
    description: "Transparent washes on cold-press paper: edge pooling, granulation, glazing, blooms and splatter.",
    aspect: 1.5,
    params: &[
        choice(
            "palette",
            "Palette",
            "cobalt",
            &[
                opt("cobalt", "Cobalt"),
                opt("sunset", "Sunset"),
                opt("forest", "Forest"),
                opt("rose", "Rose"),
                opt("sepia", "Sepia"),
            ],
        ),
        number("washes", "Washes", 2.0, 12.0, 1.0, 6.0, ""),
        number("layers", "Layers", 10.0, 40.0, 1.0, 24.0, ""),
        number("bleed", "Bleed", 0.0, 1.0, 0.01, 0.5, "%"),
        number("splatter", "Splatter", 0.0, 1.0, 0.01, 0.35, "%"),
    ],
    render: render_watercolor,
};

// ---------------- wet on wet ----------------
fn ww(id: &str) -> &'static [([f64; 3], f64)] {
    match id {
        "ocean" => &[
            ([20.0, 70.0, 150.0], 0.8),
            ([20.0, 140.0, 160.0], 0.5),
            ([80.0, 170.0, 170.0], 0.4),
            ([30.0, 40.0, 110.0], 0.9),
            ([120.0, 180.0, 200.0], 0.3),
        ],
        "bloom" => &[
            ([215.0, 50.0, 100.0], 0.2),
            ([240.0, 120.0, 70.0], 0.2),
            ([140.0, 60.0, 160.0], 0.5),
            ([250.0, 190.0, 70.0], 0.1),
            ([70.0, 90.0, 180.0], 0.6),
        ],
        "earth" => &[
            ([150.0, 80.0, 40.0], 0.8),
            ([190.0, 130.0, 50.0], 0.6),
            ([80.0, 90.0, 50.0], 0.5),
            ([110.0, 50.0, 40.0], 0.8),
            ([60.0, 70.0, 100.0], 0.9),
        ],
        _ => &[
            ([60.0, 50.0, 150.0], 0.8),
            ([200.0, 70.0, 110.0], 0.3),
            ([235.0, 140.0, 70.0], 0.2),
            ([40.0, 100.0, 160.0], 0.6),
            ([245.0, 200.0, 90.0], 0.1),
        ],
    }
}

fn render_wet(seed: &str, w: usize, h: usize, p: &Params) -> Vec<u8> {
    let mut rr = Rand::new(seed, "wet-on-wet");
    let s_ = (rr.r() * 1e6).floor();
    let (wf, hf) = (w as f64, h as f64);
    let asp = wf / hf;
    let pal = mk_pal(ww(p.str("palette")));
    let n = round(p.num("washes")) as i64;
    let flow = p.num("flow");
    let water = p.num("water");
    let bloom_p = p.num("bloom");
    let ph = paper_height(w, h, s_, 1.0);
    let pl = paper_light(&ph, w, h, 1.1);
    let mut ab = vec![0.0f64; w * h * 3];
    let wx = rr.r() * asp;
    let wy = rr.r();
    let wet_r = 0.5 + 0.6 * water;
    for k in 0..n {
        let kf = k as f64;
        let cx = asp * (0.08 + 0.84 * rr.r());
        let cy = 0.08 + 0.84 * rr.r();
        let r = 0.18 + 0.24 * rr.r() + (if k < 2 { 0.12 } else { 0.0 });
        let (pig, gr) = pal[(rr.r() * pal.len() as f64).floor() as usize];
        let ox = rr.r() * 50.0;
        let oy = rr.r() * 50.0;
        let soft = 0.03 + 0.16 * water;
        let mut a = vec![0.0f64; w * h];
        let sharp = rr.r() < 0.5;
        for y in 0..h {
            let v = y as f64 / hf;
            for x in 0..w {
                let u = x as f64 / hf;
                let wu = (fbm2(u * 1.3 + ox, v * 1.3 + oy, s_ + kf, 4) - 0.5) * flow * 0.6;
                let wv = (fbm2(u * 1.3 + oy, v * 1.3 + ox, s_ + kf + 40.0, 4) - 0.5) * flow * 0.6;
                let dx = (u + wu - cx) / r;
                let dy = (v + wv - cy) / r;
                let q = (dx * dx + dy * dy).sqrt();
                let nz = fbm2(u * 3.0 + ox, v * 3.0 + oy, s_ + kf * 7.0, 6);
                let wet = clamp(1.0 - (hypot2(u - wx, v - wy) / wet_r / asp) * 1.4, 0.0, 1.0);
                let f = 1.0 - q + (nz - 0.5) * (0.7 + 0.6 * flow);
                let s = soft * (0.4 + 1.6 * wet) * (if sharp { 0.6 } else { 1.0 });
                a[y * w + x] = sstep(-s, s * 0.6, f) * (0.55 + 0.45 * sstep(0.0, 0.5, f));
            }
        }
        let bloom = if rr.r() < bloom_p { 1.0 } else { 0.0 };
        let bloom_f = 3.0 + 3.0 * rr.r();
        let bloom_t = 0.55 + 0.08 * rr.r();
        let strength = 0.6 + 0.5 * rr.r();
        pigment_layer(
            &mut ab,
            &a,
            w,
            h,
            pig,
            &PigmentOpts {
                s: s_ + kf * 17.0,
                ph: Some(&ph),
                gran: gr * 0.55,
                edge: bloom_p * 2.6 * (if sharp { 1.0 } else { 0.4 }),
                edge_r: 0.004 + 0.006 * water,
                mottle_f: 2.5,
                bloom,
                bloom_f,
                bloom_t,
                strength,
            },
        );
    }
    develop_paper(&ab, w, h, lin3([248.0, 244.0, 234.0]), &pl)
}

pub const WET_ON_WET: Generator = Generator {
    id: "wet-on-wet",
    title: "Wet on wet",
    description:
        "Pigment dropped onto soaking paper: feathered fronts, mixing washes, granulation and cauliflower blooms.",
    aspect: 1.5,
    params: &[
        choice(
            "palette",
            "Palette",
            "dusk",
            &[
                opt("dusk", "Dusk"),
                opt("ocean", "Ocean"),
                opt("bloom", "Bloom"),
                opt("earth", "Earth"),
            ],
        ),
        number("washes", "Washes", 2.0, 8.0, 1.0, 5.0, ""),
        number("flow", "Flow", 0.0, 1.0, 0.01, 0.55, "%"),
        number("water", "Water", 0.0, 1.0, 0.01, 0.55, "%"),
        number("bloom", "Bloom", 0.0, 1.0, 0.01, 0.45, "%"),
    ],
    render: render_wet,
};

// ---------------- paste paper ----------------
fn pc(id: &str) -> ([f64; 3], [f64; 3]) {
    match id {
        "terracotta" => ([160.0, 58.0, 34.0], [242.0, 228.0, 204.0]),
        "forest" => ([26.0, 84.0, 62.0], [236.0, 232.0, 210.0]),
        "plum" => ([92.0, 32.0, 86.0], [240.0, 226.0, 222.0]),
        "ochre" => ([190.0, 130.0, 30.0], [244.0, 236.0, 220.0]),
        _ => ([24.0, 40.0, 110.0], [236.0, 230.0, 214.0]),
    }
}
fn groove(f: f64, jit: f64) -> (f64, f64) {
    let q = (f - f.floor() - 0.5).abs();
    let gw = 0.16 + jit;
    let g0 = 1.0 - sstep(gw - 0.05, gw + 0.03, 0.5 - q);
    let rd = (0.5 - q - gw - 0.04) / 0.05;
    (g0, exp(-rd * rd))
}

fn render_paste(seed: &str, w: usize, h: usize, p: &Params) -> Vec<u8> {
    let mut rr = Rand::new(seed, "paste-paper");
    let s_ = (rr.r() * 1e6).floor();
    let (wf, hf) = (w as f64, h as f64);
    let m = jmin(wf, hf);
    let (pc0, pc1) = pc(p.str("colour"));
    let pig = lin3(pc0);
    let paper = lin3(pc1);
    let pat = p.str("pattern");
    let ff = p.num("comb");
    let aa = p.num("wave");
    let streak = p.num("streak");
    let ox = rr.r() * 40.0;
    rr.r();
    let ph = rr.r() * TAU;
    let ph2 = rr.r() * TAU;
    let rot = (rr.r() - 0.5) * 0.35;
    let cr = cos(rot);
    let sr = sin(rot);
    let mut cxs = Vec::new();
    let mut cys = Vec::new();
    for _ in 0..5 {
        cxs.push((rr.r() * wf) / m);
        cys.push((rr.r() * hf) / m);
    }
    let mut tt = vec![0.0f64; w * h];
    for y in 0..h {
        for x in 0..w {
            let u = x as f64 / m;
            let v = y as f64 / m;
            let bx = u * cr + v * sr;
            let by = -u * sr + v * cr;
            let brush = 0.85
                + 0.42
                    * (vnoise(bx * 2.5 + ox, by * 90.0, s_) * 0.6 + vnoise(bx * 9.0, by * 260.0, s_ + 1.0) * 0.4 - 0.5)
                    * streak
                    * 2.0;
            let jit = (fbm2(u * 6.0, v * 6.0, s_ + 3.0, 3) - 0.5) * 0.14
                + (fbm2(u * 30.0, v * 30.0, s_ + 4.0, 2) - 0.5) * 0.05;
            let g1;
            let r1;
            let phase;
            if pat == "waves" {
                phase = (by
                    + aa * (0.06 * sin(bx * 7.0 + ph) + 0.035 * sin(bx * 15.0 + ph2))
                    + (fbm2(u * 1.5, v * 1.5, s_ + 5.0, 3) - 0.5) * aa * 0.12)
                    * ff;
                (g1, r1) = groove(phase, jit);
            } else if pat == "basket" {
                let p1 = (bx + aa * 0.03 * sin(by * 9.0 + ph)) * ff;
                let p2 = (by + aa * 0.03 * sin(bx * 9.0 + ph2)) * ff;
                let blk = ((bx * ff) / 4.0).floor() as i64 + ((by * ff) / 4.0).floor() as i64;
                phase = if (blk & 1) != 0 { p1 } else { p2 };
                (g1, r1) = groove(phase, jit);
            } else if pat == "rings" {
                let mut s = 0.0;
                for k in 0..5 {
                    s += exp(-hypot2(u - cxs[k], v - cys[k]) * 16.0);
                }
                let d = -ln(s + 1e-9) / 16.0 + (fbm2(u * 3.0, v * 3.0, s_ + 7.0, 3) - 0.5) * aa * 0.1;
                phase = d * ff * 1.4;
                (g1, r1) = groove(phase, jit);
            } else {
                phase = 0.0;
                let g = ff * 0.5;
                let uu = bx * g + (fbm2(u * 2.0, v * 2.0, s_ + 8.0, 2) - 0.5) * aa;
                let vv = by * g + (fbm2(u * 2.0 + 5.0, v * 2.0, s_ + 9.0, 2) - 0.5) * aa;
                let fx = uu.floor();
                let fy = vv.floor();
                let mut dm = 9.0;
                for j in -1..=1 {
                    for q in -1..=1 {
                        let (jf, qf) = (j as f64, q as f64);
                        let px = fx + qf + 0.25 + 0.5 * hash2(fx + qf, fy + jf, s_);
                        let py = fy + jf + 0.25 + 0.5 * hash2(fx + qf, fy + jf, s_ + 3.0);
                        let dd = hypot2(uu - px, vv - py);
                        if dd < dm {
                            dm = dd;
                        }
                    }
                }
                let rr2 = 0.27 + jit;
                let inner = 1.0 - sstep(rr2 - 0.03, rr2 + 0.01, dm);
                let vein = fbm2(u * 60.0, v * 60.0, s_ + 12.0, 3);
                g1 = inner * (0.55 + 0.45 * sstep(0.4, 0.62, vein));
                let z = (dm - rr2 - 0.035) / 0.03;
                r1 = exp(-z * z);
            }
            let sq = 0.5 + 0.5 * sin(phase * TAU * 7.0 + fbm2(u * 20.0, v * 20.0, s_ + 13.0, 2) * 4.0);
            let st = sq * sq * (0.5 + vnoise(phase * 40.0, v * 3.0, s_ + 14.0));
            let t = brush * (1.0 - 0.8 * g1) + 0.38 * r1 + g1 * 0.22 * st;
            tt[y * w + x] = jmax(0.0, t);
        }
    }
    let tb = blur(&tt, w, h, jmax(1.0, round(m * 0.0012)) as i64);
    let p_h = paper_height(w, h, s_, 1.0);
    let pl = paper_light(&p_h, w, h, 0.8);
    let k = [-ln(pig[0]), -ln(pig[1]), -ln(pig[2])];
    let mut d = new_image(w, h);
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let t = tb[i];
            let xm = if x > 0 { tb[i - 1] } else { t };
            let ym = if y > 0 { tb[i - w] } else { t };
            let xp = if x < w - 1 { tb[i + 1] } else { t };
            let yp = if y < h - 1 { tb[i + w] } else { t };
            let nx = (xm - xp) * 3.5;
            let ny = (ym - yp) * 3.5;
            let nl = hypot3(nx, ny, 1.0);
            let lam = 0.86 + ((-nx * 0.6 - ny * 0.75) / nl) * 0.5;
            let sh = pow(clamp((-nx * 0.5 - ny * 0.6 + 0.62) / nl, 0.0, 1.0), 40.0) * 0.08 * t;
            let dd = t
                * 0.82
                * (0.9 + 0.2 * p_h[i])
                * (0.85 + 0.3 * fbm2((x as f64 / m) * 4.0, (y as f64 / m) * 4.0, s_ + 20.0, 4));
            put(
                &mut d,
                i,
                enc((paper[0] * exp(-k[0] * dd) * lam + sh) * pl[i]),
                enc((paper[1] * exp(-k[1] * dd) * lam + sh) * pl[i]),
                enc((paper[2] * exp(-k[2] * dd) * lam + sh) * pl[i]),
            );
        }
    }
    d
}

pub const PASTE_PAPER: Generator = Generator {
    id: "paste-paper",
    title: "Paste paper",
    description: "Bookbinder's endpaper: coloured starch paste combed, swirled or stamped while wet, lit as a relief.",
    aspect: 1.5,
    params: &[
        choice(
            "pattern",
            "Pattern",
            "waves",
            &[
                opt("waves", "Waves"),
                opt("basket", "Basket"),
                opt("rings", "Rings"),
                opt("stamped", "Stamped"),
            ],
        ),
        choice(
            "colour",
            "Colour",
            "indigo",
            &[
                opt("indigo", "Indigo"),
                opt("terracotta", "Terracotta"),
                opt("forest", "Forest"),
                opt("plum", "Plum"),
                opt("ochre", "Ochre"),
            ],
        ),
        number("comb", "Comb", 3.0, 22.0, 0.5, 10.0, ""),
        number("wave", "Wave", 0.0, 1.5, 0.01, 0.8, "%"),
        number("streak", "Brush", 0.0, 1.0, 0.01, 0.5, "%"),
    ],
    render: render_paste,
};

// ---------------- dendritic painting ----------------
struct Dp {
    paper: [f64; 3],
    inks: &'static [[f64; 3]],
    dark: bool,
}
fn dp(id: &str) -> Dp {
    match id {
        "ink" => Dp {
            paper: [244.0, 238.0, 226.0],
            inks: &[[22.0, 20.0, 24.0], [40.0, 30.0, 26.0]],
            dark: false,
        },
        "ember" => Dp {
            paper: [18.0, 16.0, 20.0],
            inks: &[[255.0, 120.0, 30.0], [255.0, 200.0, 60.0], [220.0, 40.0, 50.0]],
            dark: true,
        },
        "spectrum" => Dp {
            paper: [246.0, 243.0, 236.0],
            inks: &[
                [230.0, 40.0, 90.0],
                [250.0, 170.0, 30.0],
                [20.0, 150.0, 170.0],
                [60.0, 60.0, 170.0],
            ],
            dark: false,
        },
        "malachite" => Dp {
            paper: [238.0, 234.0, 222.0],
            inks: &[[10.0, 90.0, 70.0], [30.0, 140.0, 100.0], [10.0, 50.0, 60.0]],
            dark: false,
        },
        _ => Dp {
            paper: [246.0, 244.0, 238.0],
            inks: &[[16.0, 48.0, 140.0], [20.0, 120.0, 150.0], [60.0, 30.0, 120.0]],
            dark: false,
        },
    }
}

const BS: i64 = 8;
struct Dla {
    gw: i64,
    gh: i64,
    bw: i64,
    bh: i64,
    occ: Vec<i64>,
    near: Vec<u8>,
    bnear: Vec<u8>,
    px: Vec<i64>,
    py: Vec<i64>,
    par: Vec<i64>,
    col: Vec<usize>,
}
impl Dla {
    fn occupy(&mut self, i: i64, pr: i64, c: usize) {
        let x = i % self.gw;
        let y = i / self.gw;
        let bx = x / BS;
        let by = y / BS;
        for j in -1..=1 {
            for q in -1..=1 {
                let xx = bx + q;
                let yy = by + j;
                if xx >= 0 && yy >= 0 && xx < self.bw && yy < self.bh {
                    self.bnear[(yy * self.bw + xx) as usize] = 1;
                }
            }
        }
        self.occ[i as usize] = self.px.len() as i64;
        self.px.push(x);
        self.py.push(y);
        self.par.push(pr);
        self.col.push(c);
        if x > 0 {
            self.near[(i - 1) as usize] = 1;
        }
        if x < self.gw - 1 {
            self.near[(i + 1) as usize] = 1;
        }
        if y > 0 {
            self.near[(i - self.gw) as usize] = 1;
        }
        if y < self.gh - 1 {
            self.near[(i + self.gw) as usize] = 1;
        }
    }
}

fn render_dendritic(seed: &str, w: usize, h: usize, p: &Params) -> Vec<u8> {
    let mut rr = Rand::new(seed, "dendritic");
    let s_ = (rr.r() * 1e6).floor();
    let pal = dp(p.str("palette"));
    let (wf, hf) = (w as f64, h as f64);
    let cs = jmax(2.0, wf / 260.0);
    let gw = round(wf / cs) as i64;
    let gh = round(hf / cs) as i64;
    let nn = gw * gh;
    let bw = (gw as f64 / BS as f64).ceil() as i64;
    let bh = (gh as f64 / BS as f64).ceil() as i64;
    let mut g = Dla {
        gw,
        gh,
        bw,
        bh,
        occ: vec![-1; nn as usize],
        near: vec![0; nn as usize],
        bnear: vec![0; (bw * bh) as usize],
        px: Vec::new(),
        py: Vec::new(),
        par: Vec::new(),
        col: Vec::new(),
    };
    let org = p.str("origin");
    let n_i = pal.inks.len();
    if org == "edges" {
        let n = 3 + (rr.r() * 4.0).floor() as i64;
        for s in 0..n {
            let e = (rr.r() * 4.0).floor() as i64;
            let t = 0.1 + 0.8 * rr.r();
            let (x, y) = match e {
                0 => (t * gw as f64, 0.0),
                1 => (t * gw as f64, (gh - 1) as f64),
                2 => (0.0, t * gh as f64),
                _ => ((gw - 1) as f64, t * gh as f64),
            };
            let i = y.floor() as i64 * gw + x.floor() as i64;
            if g.occ[i as usize] < 0 {
                g.occupy(i, -1, s as usize % n_i);
            }
        }
    } else {
        let centre = org == "centre";
        let n = if centre {
            1 + (rr.r() * 3.0).floor() as i64
        } else {
            6 + (rr.r() * 8.0).floor() as i64
        };
        let lo = if centre { 0.35 } else { 0.05 };
        let sp = if centre { 0.3 } else { 0.9 };
        let lox = if centre { 0.3 } else { 0.05 };
        let spx = if centre { 0.4 } else { 0.9 };
        for s in 0..n {
            let yy = (gh as f64 * (lo + sp * rr.r())).floor() as i64;
            let xx = (gw as f64 * (lox + spx * rr.r())).floor() as i64;
            let i = yy * gw + xx;
            if g.occ[i as usize] < 0 {
                g.occupy(i, -1, s as usize % n_i);
            }
        }
    }
    let target = round(nn as f64 * p.num("fill") * 0.16) as i64;
    let stick = 0.3 + 0.7 * p.num("branch");
    let mut guard: i64 = 0;
    while (g.px.len() as i64) < target && {
        let gv = guard;
        guard += 1;
        gv < target * 40
    } {
        let mut x = (rr.r() * gw as f64).floor() as i64;
        let mut y = (rr.r() * gh as f64).floor() as i64;
        let mut i = y * gw + x;
        if g.occ[i as usize] >= 0 {
            continue;
        }
        for _ in 0..9000 {
            if g.near[i as usize] != 0 && rr.r() < stick {
                let mut pr = -1;
                for j in [i - 1, i + 1, i - gw, i + gw] {
                    if j >= 0 && j < nn && g.occ[j as usize] >= 0 && ((j % gw) - x).abs() <= 1 {
                        pr = g.occ[j as usize];
                        break;
                    }
                }
                if pr >= 0 {
                    let c = g.col[pr as usize];
                    g.occupy(i, pr, c);
                    break;
                }
            }
            if g.bnear[((y / BS) * bw + (x / BS)) as usize] == 0 {
                let a = rr.r() * TAU;
                x = round(x as f64 + cos(a) * 6.0) as i64;
                y = round(y as f64 + sin(a) * 6.0) as i64;
            } else {
                let r = (rr.r() * 4.0).floor() as i64;
                match r {
                    0 => x += 1,
                    1 => x -= 1,
                    2 => y += 1,
                    _ => y -= 1,
                }
            }
            if x < 0 || y < 0 || x >= gw || y >= gh {
                break;
            }
            i = y * gw + x;
            if g.occ[i as usize] >= 0 {
                break;
            }
        }
    }
    let n = g.px.len();
    let mut cnt = vec![1.0f64; n];
    for k in (0..n).rev() {
        if g.par[k] >= 0 {
            let pk = g.par[k] as usize;
            cnt[pk] += cnt[k];
        }
    }
    let jx = |k: usize| -> f64 {
        (g.px[k] as f64 + 0.5 + (hash2(g.px[k] as f64, g.py[k] as f64, s_ + 1.0) - 0.5) * 0.95) * cs
    };
    let jy = |k: usize| -> f64 {
        (g.py[k] as f64 + 0.5 + (hash2(g.px[k] as f64, g.py[k] as f64, s_ + 2.0) - 0.5) * 0.95) * cs
    };
    let mut th = vec![0.0f64; w * h];
    let mut cw: Vec<Vec<f64>> = Vec::new();
    let mut mask = Mask::new(w, h);
    let wt = p.num("weight");
    for c in 0..n_i {
        let mut t = vec![0.0f64; w * h];
        for k in 0..n {
            if g.col[k] != c || g.par[k] < 0 {
                continue;
            }
            let q = g.par[k] as usize;
            capsule(
                &mut mask,
                jx(k),
                jy(k),
                jx(q),
                jy(q),
                cs * (0.45 + 0.42 * ln(1.0 + cnt[k])) * wt,
            );
        }
        for y in mask.y0..=mask.y1 {
            for x in mask.x0..=mask.x1 {
                let i = y as usize * w + x as usize;
                t[i] = jmin(1.0, mask.cov[i]);
                th[i] += t[i];
                mask.cov[i] = 0.0;
            }
        }
        mask.reset_box();
        cw.push(t);
    }
    let br = jmax(1.0, round(cs * 0.8)) as i64;
    let tb = blur(&blur(&th, w, h, br), w, h, br);
    let ph = paper_height(w, h, s_, 1.0);
    let pl = paper_light(&ph, w, h, 0.9);
    let pp = lin3(pal.paper);
    let inks: Vec<V3> = pal.inks.iter().map(|c| lin3(*c)).collect();
    let stain = blur(&th, w, h, round(cs * 5.0) as i64);
    let mut d = new_image(w, h);
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let mut r = pp[0] * pl[i];
            let mut gg = pp[1] * pl[i];
            let mut b = pp[2] * pl[i];
            let sum = th[i];
            let cov = clamp(sum, 0.0, 1.0);
            let halo = clamp(stain[i] * 1.2, 0.0, 1.0) * (1.0 - cov) * 0.12;
            if !pal.dark {
                r *= 1.0 - halo * 0.6;
                gg *= 1.0 - halo * 0.7;
                b *= 1.0 - halo * 0.95;
            }
            if cov > 0.0 {
                let mut ir = 0.0;
                let mut ig = 0.0;
                let mut ib = 0.0;
                for c in 0..n_i {
                    let wv = cw[c][i] / (sum + 1e-6);
                    ir += inks[c][0] * wv;
                    ig += inks[c][1] * wv;
                    ib += inks[c][2] * wv;
                }
                let thick = clamp(tb[i] * 1.15, 0.0, 1.4);
                let edge = clamp(cov - tb[i] * 0.9, 0.0, 1.0);
                let xm = if x > 0 { tb[i - 1] } else { tb[i] };
                let ym = if y > 0 { tb[i - w] } else { tb[i] };
                let xp = if x < w - 1 { tb[i + 1] } else { tb[i] };
                let yp = if y < h - 1 { tb[i + w] } else { tb[i] };
                let nx = (xm - xp) * 1.6;
                let ny = (ym - yp) * 1.6;
                let nl = hypot3(nx, ny, 1.0);
                let lam = clamp(0.78 + ((-nx * 0.55 - ny * 0.7) / nl) * 0.6, 0.0, 1.4);
                let spec = pow(clamp((-nx * 0.4 - ny * 0.6 + 0.7) / nl, 0.0, 1.0), 28.0) * 0.35 * thick;
                let dens = 0.55 + 0.45 * thick + edge * 0.35;
                let gran = 0.9 + 0.2 * ph[i];
                let (pr, pg, pb);
                if pal.dark {
                    let f = lam * gran * (1.1 - 0.3 * thick);
                    pr = ir * f;
                    pg = ig * f;
                    pb = ib * f;
                } else {
                    pr = pp[0] * pow(ir, dens * gran) * lam;
                    pg = pp[1] * pow(ig, dens * gran) * lam;
                    pb = pp[2] * pow(ib, dens * gran) * lam;
                }
                r = r * (1.0 - cov) + (pr + spec) * cov;
                gg = gg * (1.0 - cov) + (pg + spec) * cov;
                b = b * (1.0 - cov) + (pb + spec) * cov;
            }
            put(&mut d, i, enc(r), enc(gg), enc(b));
        }
    }
    d
}

pub const DENDRITIC: Generator = Generator {
    id: "dendritic-painting",
    title: "Dendritic painting",
    description:
        "Paint pressed between plates and pulled apart: branching fingers grown by diffusion-limited aggregation.",
    aspect: 1.5,
    params: &[
        choice(
            "palette",
            "Paint",
            "cobalt",
            &[
                opt("ink", "Ink"),
                opt("cobalt", "Cobalt"),
                opt("ember", "Ember"),
                opt("spectrum", "Spectrum"),
                opt("malachite", "Malachite"),
            ],
        ),
        choice(
            "origin",
            "Origin",
            "edges",
            &[
                opt("edges", "Edges"),
                opt("centre", "Centre"),
                opt("scatter", "Scatter"),
            ],
        ),
        number("fill", "Growth", 0.2, 1.0, 0.01, 0.85, "%"),
        number("branch", "Branching", 0.0, 1.0, 0.01, 0.7, "%"),
        number("weight", "Weight", 0.6, 2.0, 0.02, 1.0, "x"),
    ],
    render: render_dendritic,
};
