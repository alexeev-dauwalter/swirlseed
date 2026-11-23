//! Aurora: folded emitting sheets 100–300 km up, seen in perspective from the ground, over stars.
//! Mirrors `aurora.ts`.

use super::core::*;
use super::spec::*;

struct Col {
    fr: V3,
    lo: V3,
    hi: V3,
    top: V3,
}

fn col(id: &str) -> Col {
    match id {
        "violet" => Col {
            fr: [0.9, 0.5, 1.0],
            lo: [0.55, 0.3, 1.0],
            hi: [0.3, 0.35, 1.0],
            top: [0.25, 0.15, 0.7],
        },
        "crimson" => Col {
            fr: [1.0, 0.5, 0.5],
            lo: [1.0, 0.16, 0.24],
            hi: [0.8, 0.12, 0.4],
            top: [0.4, 0.08, 0.4],
        },
        "mixed" => Col {
            fr: [1.0, 0.3, 0.7],
            lo: [0.15, 1.0, 0.45],
            hi: [0.1, 0.7, 0.5],
            top: [0.9, 0.15, 0.35],
        },
        _ => Col {
            fr: [0.5, 0.9, 0.6],
            lo: [0.12, 1.0, 0.38],
            hi: [0.08, 0.55, 0.42],
            top: [0.55, 0.12, 0.35],
        },
    }
}

fn render(seed: &str, w: usize, h: usize, p: &Params) -> Vec<u8> {
    let mut rr = Rand::new(seed, "aurora");
    let s_ = (rr.r() * 1e6).floor();
    let cc = col(p.str("colour"));
    let nc = round(p.num("curtains")) as i64;
    let rays = p.num("rays");
    let inten = p.num("intensity");
    let n_ = w * h;
    let (wf, hf) = (w as f64, h as f64);
    let (wi, hi) = (w as i64, h as i64);
    let mut er = vec![0.0f64; n_];
    let mut eg = vec![0.0f64; n_];
    let mut eb = vec![0.0f64; n_];
    let f = hf * 0.85;
    let hor = hf * 1.04;
    let cx = wf / 2.0;
    for c in 0..nc {
        let cf = c as f64;
        let off = rr.r() * 50.0;
        let zc = if c == 0 {
            110.0 + rr.r() * 90.0
        } else {
            220.0 + rr.r() * 500.0
        };
        let phi0 = (rr.r() - 0.5) * 0.8;
        let bend = 1.6 + 1.8 * rr.r();
        let len = zc * (3.2 + 1.6 * rr.r());
        let m = round(wf * 14.0) as usize;
        let mf = m as f64;
        let top = 70.0 + 120.0 * rr.r();
        let curl = 6.0 + 10.0 * rr.r();
        let near = 320.0 / zc;
        let mut xa = vec![0.0f64; m];
        let mut za = vec![0.0f64; m];
        {
            let mut x = 0.0;
            let mut z = zc;
            let mut sx = 0.0;
            for k in 0..m {
                let ph = phi0 + bend * (fbm2((k as f64 / mf) * 2.2 + off, cf, s_ + cf, 3) - 0.5) * 2.0;
                x += (cos(ph) * len) / mf;
                z += ((sin(ph) * len) / mf) * 0.6;
                xa[k] = x;
                za[k] = jmax(z, 30.0);
                if z >= 100.0 {
                    sx += x / z;
                }
            }
            let c0 = sx / mf - (rr.r() - 0.5) * 0.35;
            for k in 0..m {
                xa[k] -= c0 * za[k];
            }
        }
        for k in 0..m {
            let s = k as f64 / mf;
            let xx = xa[k];
            let z = za[k];
            if z < 100.0 {
                continue;
            }
            let xc = xx
                + curl
                    * (fbm2(s * 38.0 + off, cf + 3.0, s_ + cf + 5.0, 3) - 0.5)
                    * 2.0
                    * (0.4 + fbm2(s * 4.0 + off, cf, s_ + cf + 6.0, 2));
            let x = cx + (xc / z) * f;
            if x < -2.0 || x >= wf + 1.0 {
                continue;
            }
            let a0 = 100.0 + 8.0 * (fbm2(s * 9.0 + off, cf + 1.0, s_ + cf + 7.0, 2) - 0.5);
            let a1 = a0
                + top
                    * (0.45 + 0.9 * fbm2(s * 6.0 + off, cf + 2.0, s_ + cf + 8.0, 3))
                    * (0.7 + 0.6 * vnoise(s * rays * 3.0 + off, cf, s_ + cf + 9.0));
            let yb = hor - (a0 * f) / z;
            let hh = ((a1 - a0) * f) / z;
            let ray = pow(
                clamp(
                    vnoise(s * rays * 4.0 + off, cf, s_ + cf + 60.0) * 0.6
                        + vnoise(s * rays * 13.0 + off, cf + 0.5, s_ + cf + 61.0) * 0.4,
                    0.0,
                    1.0,
                ),
                2.4,
            );
            let act = pow(
                clamp(fbm2(s * 3.2 + off, cf + 7.0, s_ + cf + 80.0, 4) * 2.0 - 0.35, 0.0, 1.0),
                1.1,
            ) * sstep(0.0, 0.08, s)
                * sstep(1.0, 0.92, s);
            let ext = exp(-jmax(0.0, z - 300.0) / 1400.0);
            let ii = (0.12 + 2.0 * ray)
                * act
                * inten
                * ext
                * ((len / mf / z) * f * 0.3)
                * (if near > 1.0 { 1.0 } else { 0.75 + 0.25 * near });
            if ii < 1e-6 {
                continue;
            }
            let xi = x.floor() as i64;
            let fx = x - xi as f64;
            let y0 = jmax(0.0, (yb - hh * 1.7).floor()) as i64;
            let y1 = jmin((h - 1) as f64, (yb + 3.0).ceil()) as i64;
            let mut y = y0;
            while y <= y1 {
                let z = (yb - y as f64) / hh;
                let e = if z < 0.0 {
                    exp(z * hh * 0.8)
                } else {
                    (exp(-z * 2.6) * 0.85 + 0.15 * exp(-z * 0.7))
                        * (1.0 - exp(-z * hh * 0.45 - 0.3))
                        * (1.0 + 0.9 * exp(-z * hh * 0.1))
                        * (1.0 - sstep(0.45, 1.65, z))
                };
                if e < 0.003 {
                    y += 1;
                    continue;
                }
                let tl = clamp(z / 0.7, 0.0, 1.0);
                let fr = exp(-jmax(0.0, z) * hh * 0.3) * 0.55;
                let tp = sstep(0.35, 1.0, z);
                let mut r = lerp(lerp(lerp(cc.lo[0], cc.hi[0], tl), cc.top[0], tp), cc.fr[0], fr * 0.5);
                let mut g = lerp(lerp(lerp(cc.lo[1], cc.hi[1], tl), cc.top[1], tp), cc.fr[1], fr * 0.5);
                let mut b = lerp(lerp(lerp(cc.lo[2], cc.hi[2], tl), cc.top[2], tp), cc.fr[2], fr * 0.5);
                let wt = e * ii;
                r *= wt;
                g *= wt;
                b *= wt;
                let i0 = y * wi + xi;
                if xi >= 0 && xi < wi {
                    let i = i0 as usize;
                    er[i] += r * (1.0 - fx);
                    eg[i] += g * (1.0 - fx);
                    eb[i] += b * (1.0 - fx);
                }
                if xi + 1 < wi && xi + 1 >= 0 {
                    let i = (i0 + 1) as usize;
                    er[i] += r * fx;
                    eg[i] += g * fx;
                    eb[i] += b * fx;
                }
                y += 1;
            }
        }
    }
    let _ = hi;
    let sm = jmax(1.0, round(wf / 700.0)) as i64;
    let gl = round(wf / 40.0) as i64;
    let ar = blur(&er, w, h, sm);
    let ag = blur(&eg, w, h, sm);
    let ab = blur(&eb, w, h, sm);
    let gr = blur(&blur(&er, w, h, gl), w, h, gl);
    let gg = blur(&blur(&eg, w, h, gl), w, h, gl);
    let gb = blur(&blur(&eb, w, h, gl), w, h, gl);
    let mut st = vec![0.0f64; n_];
    let ns = round((p.num("stars") * n_ as f64) / 700.0) as i64;
    for _ in 0..ns {
        let x = (rr.r() * wf).floor() as usize;
        let y = (rr.r() * hf).floor() as usize;
        let b = pow(rr.r(), 4.5) * 1.6 + 0.05;
        st[y * w + x] += b * (1.0 - sstep(0.6, 1.0, y as f64 / hf) * 0.8);
    }
    let st_b = blur(&st, w, h, 1);
    let mut d = new_image(w, h);
    for y in 0..h {
        let v = y as f64 / hf;
        for x in 0..w {
            let i = y * w + x;
            let mut r = 0.003 + 0.006 * v * v * v;
            let mut g = 0.005 + 0.01 * pow(v, 2.6);
            let mut b = 0.014 + 0.022 * pow(v, 2.2);
            let s = st[i] * 0.85 + st_b[i] * 0.6;
            r += s + ar[i] + gr[i] * 0.35;
            g += s + ag[i] + gg[i] * 0.35;
            b += s * 1.05 + ab[i] + gb[i] * 0.35;
            let n = (hash2(x as f64, y as f64, s_) - 0.5) * 0.004;
            put(
                &mut d,
                i,
                enc(1.0 - exp(-(r + n) * 1.4)),
                enc(1.0 - exp(-(g + n) * 1.4)),
                enc(1.0 - exp(-(b + n) * 1.4)),
            );
        }
    }
    d
}

pub const AURORA: Generator = Generator {
    id: "aurora",
    title: "Aurora",
    description:
        "Northern lights: thin glowing curtains that fold back on themselves, in perspective over a starry sky.",
    aspect: 1.6,
    params: &[
        choice(
            "colour",
            "Colour",
            "green",
            &[
                opt("green", "Green"),
                opt("violet", "Violet"),
                opt("crimson", "Crimson"),
                opt("mixed", "Mixed"),
            ],
        ),
        number("intensity", "Intensity", 0.4, 2.0, 0.02, 1.0, "%"),
        number("curtains", "Curtains", 1.0, 3.0, 1.0, 2.0, ""),
        number("rays", "Rays", 20.0, 140.0, 1.0, 70.0, ""),
        number("stars", "Stars", 0.0, 1.0, 0.01, 0.7, "%"),
    ],
    render,
};
