//! Ebru and suminagashi. Mirrors `marble.ts`.

use super::core::*;
use super::raster::{capsule, fill_polygon, paint, Mask};
use super::spec::*;

const LN2_: f64 = std::f64::consts::LN_2;

struct Blob {
    x: Vec<f64>,
    y: Vec<f64>,
    c: V3,
    m: u8,
}

struct Marble {
    p: Vec<Blob>,
    seg: f64,
    cap: usize,
}

impl Marble {
    fn new(seg: f64) -> Marble {
        Marble {
            p: Vec::new(),
            seg,
            cap: 7000,
        }
    }
    fn apply<F: Fn(f64, f64) -> (f64, f64)>(&mut self, f: F) {
        let max_l = self.seg * 1.6;
        let cap = self.cap;
        for p in self.p.iter_mut() {
            let n = p.x.len();
            let mut mx = vec![0.0; n];
            let mut my = vec![0.0; n];
            for i in 0..n {
                let (a, b) = f(p.x[i], p.y[i]);
                mx[i] = a;
                my[i] = b;
            }
            let mut nx = Vec::with_capacity(n);
            let mut ny = Vec::with_capacity(n);
            let mut budget: i64 = if cap > n { (cap - n) as i64 } else { 0 };
            for i in 0..n {
                nx.push(mx[i]);
                ny.push(my[i]);
                let j = if i + 1 < n { i + 1 } else { 0 };
                let l = hypot2(mx[j] - mx[i], my[j] - my[i]);
                if l > max_l && budget > 0 {
                    let m = jmin(jmin(12.0, (l / max_l).ceil()), (budget + 1) as f64) as i64;
                    for s in 1..m {
                        let t = s as f64 / m as f64;
                        let (a, b) = f(p.x[i] + (p.x[j] - p.x[i]) * t, p.y[i] + (p.y[j] - p.y[i]) * t);
                        nx.push(a);
                        ny.push(b);
                    }
                    budget -= m - 1;
                }
            }
            p.x = nx;
            p.y = ny;
        }
    }
    fn drop(&mut self, cx: f64, cy: f64, r: f64, c: V3, meta: u8) {
        let r2 = r * r;
        self.apply(|x, y| {
            let dx = x - cx;
            let dy = y - cy;
            let d2 = dx * dx + dy * dy + 1e-14;
            let k = (1.0 + r2 / d2).sqrt();
            (cx + dx * k, cy + dy * k)
        });
        let n = jmax(36.0, jmin(600.0, ((TAU * r) / self.seg).ceil())) as usize;
        let mut xs = Vec::with_capacity(n);
        let mut ys = Vec::with_capacity(n);
        for i in 0..n {
            let a = (i as f64 / n as f64) * TAU;
            xs.push(cx + r * cos(a));
            ys.push(cy + r * sin(a));
        }
        self.p.push(Blob {
            x: xs,
            y: ys,
            c,
            m: meta,
        });
    }
    fn tine(&mut self, ax: f64, ay: f64, mut mx: f64, mut my: f64, z: f64, c: f64) {
        let l = hypot2(mx, my);
        mx /= l;
        my /= l;
        let nx = -my;
        let ny = mx;
        let k = LN2_ / c;
        self.apply(|x, y| {
            let d = ((x - ax) * nx + (y - ay) * ny).abs();
            let f = z * exp(-d * k);
            (x + f * mx, y + f * my)
        });
    }
    #[allow(clippy::too_many_arguments)]
    fn comb(&mut self, ax: f64, ay: f64, mut mx: f64, mut my: f64, z: f64, c: f64, sp: f64, alt: bool) {
        let l = hypot2(mx, my);
        mx /= l;
        my /= l;
        let nx = -my;
        let ny = mx;
        let k = LN2_ / c;
        self.apply(|x, y| {
            let t = (x - ax) * nx + (y - ay) * ny;
            let k0 = round(t / sp);
            let mut f = 0.0;
            let mut q = k0 - 1.0;
            while q <= k0 + 1.0 {
                let d = (t - q * sp).abs();
                let odd = (q as i64) & 1 != 0;
                f += (if alt && odd { -1.0 } else { 1.0 }) * z * exp(-d * k);
                q += 1.0;
            }
            (x + f * mx, y + f * my)
        });
    }
    fn swirl(&mut self, cx: f64, cy: f64, a: f64, rr: f64) {
        self.apply(|x, y| {
            let dx = x - cx;
            let dy = y - cy;
            let r = hypot2(dx, dy);
            let th = a * exp(-r / rr);
            let cs = cos(th);
            let sn = sin(th);
            (cx + dx * cs - dy * sn, cy + dx * sn + dy * cs)
        });
    }
    fn breath(&mut self, amp: f64, freq: f64, s: f64, oct: u32) {
        let e = 1e-3;
        let psi = |x: f64, y: f64| fbm_s(x * freq, y * freq, s, oct);
        self.apply(|x, y| {
            let vx = (psi(x, y + e) - psi(x, y - e)) / (2.0 * e);
            let vy = -(psi(x + e, y) - psi(x - e, y)) / (2.0 * e);
            (x + vx * amp, y + vy * amp)
        });
    }
}

fn mix3(a: V3, b: V3, t: f64) -> V3 {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn paper_texture(w: usize, h: usize, salt: &str, strength: f64) -> Vec<f64> {
    let mut p = vec![0.0f64; w * h * 3];
    let s = (Rand::new(salt, "paper").r() * 9973.0).floor();
    let sc = 600.0 / (w.max(h)) as f64;
    for y in 0..h {
        for x in 0..w {
            let cl = fbm_s(x as f64 * sc * 0.012, y as f64 * sc * 0.012, s, 4);
            let g = hash2(x as f64, y as f64, s + 7.0) - 0.5;
            let v = 255.0 * (1.0 - strength * (0.05 * cl + 0.035 * g + 0.02));
            let i = (y * w + x) * 3;
            p[i] = v;
            p[i + 1] = v * 0.995;
            p[i + 2] = v * 0.985;
        }
    }
    let mut r = Rand::new(salt, "fibres");
    let n = round(((w * h) as f64 / 2600.0) * strength) as i64;
    let mut m = Mask::new(w, h);
    let wf = w as f64;
    let hf = h as f64;
    for _ in 0..n {
        let x = r.r() * wf;
        let y = r.r() * hf;
        let a = r.r() * TAU;
        let l = ((6.0 + r.r() * 26.0) * wf) / 1000.0;
        let cv = (r.r() - 0.5) * l * 0.6;
        let dark = r.r() < 0.5;
        let lw = ((0.4 + r.r() * 0.7) * wf) / 1000.0;
        let ca = cos(a);
        let sa = sin(a);
        let qx = x + (ca * l) / 2.0 - sa * cv;
        let qy = y + (sa * l) / 2.0 + ca * cv;
        let ex = x + ca * l;
        let ey = y + sa * l;
        let mut px = x;
        let mut py = y;
        for k in 1..=6 {
            let t = k as f64 / 6.0;
            let u = 1.0 - t;
            let nx = u * u * x + 2.0 * u * t * qx + t * t * ex;
            let ny = u * u * y + 2.0 * u * t * qy + t * t * ey;
            capsule(&mut m, px, py, nx, ny, lw);
            px = nx;
            py = ny;
        }
        if dark {
            paint(&mut m, &mut p, 120.0, 100.0, 70.0, 0.1);
        } else {
            paint(&mut m, &mut p, 255.0, 255.0, 255.0, 0.35);
        }
    }
    p
}

fn finish(img: &[f64], paper: &[f64], w: usize, h: usize) -> Vec<u8> {
    let mut d = new_image(w, h);
    for i in 0..w * h {
        put(
            &mut d,
            i,
            (img[i * 3] * paper[i * 3]) / 255.0,
            (img[i * 3 + 1] * paper[i * 3 + 1]) / 255.0,
            (img[i * 3 + 2] * paper[i * 3 + 2]) / 255.0,
        );
    }
    d
}

fn draw_blobs(mk: &Marble, w: usize, h: usize, img: &mut [f64], outline: &dyn Fn(&Blob) -> Option<(V3, f64, f64)>) {
    let mut fill = Mask::new(w, h);
    let mut line = Mask::new(w, h);
    let wf = w as f64;
    let mut xs: Vec<f64> = Vec::new();
    let mut ys: Vec<f64> = Vec::new();
    for p in &mk.p {
        let n = p.x.len();
        xs.clear();
        ys.clear();
        for i in 0..n {
            xs.push(p.x[i] * wf);
            ys.push(p.y[i] * wf);
        }
        fill_polygon(&mut fill, &xs, &ys, n);
        paint(&mut fill, img, p.c[0], p.c[1], p.c[2], 1.0);
        if let Some((c, a, lw)) = outline(p) {
            for i in 0..n {
                let j = if i + 1 < n { i + 1 } else { 0 };
                capsule(&mut line, xs[i], ys[i], xs[j], ys[j], lw);
            }
            paint(&mut line, img, c[0], c[1], c[2], a);
        }
    }
}

fn seg_for(w: usize) -> f64 {
    if w >= 900 {
        0.0028
    } else if w >= 500 {
        0.004
    } else {
        0.006
    }
}

// ---------------- ebru ----------------
fn ebru_pal(id: &str) -> (&'static str, [&'static str; 5]) {
    match id {
        "klasik" => ("#efe6d0", ["#1f2c5c", "#c8963e", "#8c2f2b", "#5f7a5a", "#efe6d0"]),
        "deniz" => ("#e9e3d3", ["#123a63", "#2f8f9d", "#d9c9a3", "#0b1f3a", "#f1ead8"]),
        "gul" => ("#f2e7dc", ["#7a1f33", "#c9566b", "#d9b46a", "#8a9a78", "#f4ece2"]),
        "toprak" => ("#e8dcc2", ["#5b3a24", "#a0522d", "#c99a3b", "#6b6b3a", "#efe4cc"]),
        _ => ("#f0e8d4", ["#b3261e", "#e0a526", "#2e6b3f", "#2a4a9b", "#f2ead6"]),
    }
}
const EBRU_PAL_IDS: [&str; 5] = ["klasik", "deniz", "gul", "toprak", "lale"];

fn ebru_plan(seed: &str, p: &Params, h: f64, mk: &mut Marble) -> V3 {
    let mut rr = Rand::new(seed, "ebru");
    let pal_id = if p.str("palette") == "auto" {
        EBRU_PAL_IDS[(Rand::new(seed, "ebru-pal").r() * 5.0).floor() as usize]
    } else {
        p.str("palette")
    };
    let (bg, ink_hex) = ebru_pal(pal_id);
    let inks: Vec<V3> = ink_hex.iter().map(|h| hex_rgb(h)).collect();
    let style = p.str("style");
    let s = p.num("size");
    let dn = p.num("density");
    let c = p.num("comb");
    let battal = |rr: &mut Rand, mk: &mut Marble, scale: f64, count: f64| {
        let rounds = inks.len();
        for k in 0..rounds {
            let n = round(((count * (0.6 + 0.8 * rr.r())) / rounds as f64) * (if k == rounds - 1 { 1.4 } else { 1.0 }))
                as i64;
            for _ in 0..n {
                let x = -0.05 + rr.r() * 1.1;
                let y = -0.05 + rr.r() * (h + 0.1);
                let q = rr.r();
                mk.drop(x, y, (0.016 + q * q * 0.055) * scale, inks[k], 0);
            }
        }
    };
    if style == "hatip" {
        battal(&mut rr, mk, s * 0.8, round(200.0 * dn));
        let sp = 0.16 / dn.sqrt();
        let nr = 3 + (if rr.r() < 0.5 { 1 } else { 0 });
        let rings: Vec<V3> = [inks[2], inks[4], inks[0], inks[1]][..nr].to_vec();
        let cols = (1.0 / sp).ceil() as i64 + 1;
        let rows = (h / sp).ceil() as i64 + 1;
        let base = (0.034 + 0.01 * rr.r()) * s;
        for j in 0..rows {
            for i in 0..cols {
                let x = (i as f64 + 0.5 + (j & 1) as f64 * 0.5) * sp - sp * 0.25;
                let y = (j as f64 + 0.5) * sp;
                for (q, col) in rings.iter().enumerate() {
                    mk.drop(x, y, base * (1.0 - q as f64 * 0.22), *col, 0);
                }
            }
        }
        for i in 0..cols * 2 {
            mk.tine((i as f64 * 0.5 + 0.5) * sp - sp * 0.25, 0.0, 0.0, 1.0, 0.06, 0.011);
        }
        return hex_rgb(bg);
    }
    battal(&mut rr, mk, s, round(320.0 * dn));
    if style != "battal" {
        let sp = 0.075 / c;
        let a = rr.r() * sp;
        mk.comb(a, 0.0, 0.0, 1.0, 0.07, 0.012, sp, true);
        let sp2 = 0.09 / c;
        let b = rr.r() * sp2;
        mk.comb(0.0, b, 1.0, 0.0, 0.06, 0.012, sp2, true);
    }
    if style == "tarakli" || style == "bulbul-yuvasi" {
        let sp = 0.022 / c;
        let ph = rr.r() * sp;
        let dir = if rr.r() < 0.5 { 1.0 } else { -1.0 };
        mk.comb(ph, 0.0, 0.0, dir, 0.05, 0.006, sp, false);
    }
    if style == "bulbul-yuvasi" {
        let sp = 0.11 / c.sqrt();
        let dir = if rr.r() < 0.5 { 1.0 } else { -1.0 };
        let ny = (h / sp - 0.5).ceil() as i64;
        let nx = (1.0 / sp - 0.5).ceil() as i64;
        for j in 0..ny {
            for i in 0..nx {
                let xx = sp / 2.0 + i as f64 * sp + (rr.r() - 0.5) * sp * 0.15;
                let yy = sp / 2.0 + j as f64 * sp + (rr.r() - 0.5) * sp * 0.15;
                let a = dir * (9.0 + rr.r() * 4.0);
                mk.swirl(xx, yy, a, sp * 0.17);
            }
        }
    }
    hex_rgb(bg)
}

fn render_ebru(seed: &str, w: usize, h: usize, p: &Params) -> Vec<u8> {
    let mut mk = Marble::new(seg_for(w));
    let bg = ebru_plan(seed, p, h as f64 / w as f64, &mut mk);
    let mut img = vec![0.0f64; w * h * 3];
    for i in 0..w * h {
        img[i * 3] = bg[0];
        img[i * 3 + 1] = bg[1];
        img[i * 3 + 2] = bg[2];
    }
    let lw = jmax(0.6, w as f64 / 1100.0);
    draw_blobs(&mk, w, h, &mut img, &|b: &Blob| {
        Some((mix3(b.c, [0.0, 0.0, 0.0], 0.35), 0.45, lw))
    });
    let paper = paper_texture(w, h, "paper", 1.0);
    finish(&img, &paper, w, h)
}

pub const EBRU: Generator = Generator {
    id: "ebru",
    title: "Ebru",
    description: "Turkish paper marbling: drops of paint sprinkled on size, then combed, raked and swirled.",
    aspect: 1.5,
    params: &[
        choice(
            "style",
            "Style",
            "tarakli",
            &[
                opt("battal", "Battal"),
                opt("gel-git", "Gel-git"),
                opt("tarakli", "Taraklı"),
                opt("bulbul-yuvasi", "Bülbül yuvası"),
                opt("hatip", "Hatip"),
            ],
        ),
        choice(
            "palette",
            "Palette",
            "auto",
            &[
                opt("auto", "Auto"),
                opt("klasik", "Klasik"),
                opt("deniz", "Deniz"),
                opt("gul", "Gül"),
                opt("toprak", "Toprak"),
                opt("lale", "Lale"),
            ],
        ),
        number("density", "Density", 0.3, 1.6, 0.05, 1.0, "x"),
        number("size", "Drop size", 0.5, 1.8, 0.05, 1.0, "x"),
        number("comb", "Comb", 0.5, 2.0, 0.05, 1.0, "x"),
    ],
    render: render_ebru,
};

// ---------------- suminagashi ----------------
fn sumi_paper() -> V3 {
    hex_rgb("#f5f1e7")
}
fn sumi_inks(id: &str) -> Vec<V3> {
    let l: &[&str] = match id {
        "sumi" => &["#121212"],
        "ai" => &["#1d3566"],
        "sumi-ai" => &["#121212", "#1d3566"],
        _ => &["#121212", "#121212", "#121212", "#b3322a"],
    };
    l.iter().map(|h| hex_rgb(h)).collect()
}

struct Centre {
    x: f64,
    y: f64,
    n: i64,
    w: f64,
    k: i64,
    r: f64,
}

fn sumi_plan(seed: &str, p: &Params, h: f64, mk: &mut Marble) {
    let mut rr = Rand::new(seed, "sumi");
    let inks = sumi_inks(p.str("ink"));
    let paper = sumi_paper();
    let n_c = if p.num("centers") >= 1.0 {
        round(p.num("centers")) as i64
    } else {
        1 + (rr.r() * 3.0).floor() as i64
    };
    let rings = p.num("rings");
    let mut cs: Vec<Centre> = Vec::new();
    for _ in 0..n_c {
        let x = 0.12 + rr.r() * 0.76;
        let y = h * (0.12 + rr.r() * 0.76);
        let n = jmax(
            4.0,
            round((rings * (0.65 + 0.7 * rr.r())) / (n_c as f64 * 0.8 + 0.2).sqrt()),
        ) as i64;
        let w = 0.7 + 0.6 * rr.r();
        cs.push(Centre {
            x,
            y,
            n,
            w,
            k: 0,
            r: 0.0,
        });
    }
    let total: i64 = cs.iter().map(|c| c.n).sum();
    let r_base = ((0.8 * h) / (2.53 * PI * total as f64)).sqrt();
    for c in cs.iter_mut() {
        c.r = r_base * c.w;
    }
    for _ in 0..total {
        let live: Vec<usize> = (0..cs.len()).filter(|&i| cs[i].k < cs[i].n).collect();
        if live.is_empty() {
            break;
        }
        let ci = live[(rr.r() * live.len() as f64).floor() as usize];
        for pass in 0..2 {
            let x = cs[ci].x + (rr.r() - 0.5) * 0.003;
            let y = cs[ci].y + (rr.r() - 0.5) * 0.003;
            if pass == 0 {
                let base = inks[(rr.r() * inks.len() as f64).floor() as usize];
                let dens = 0.45 + 0.55 * pow(rr.r(), 0.6);
                let r = cs[ci].r * (0.75 + 0.55 * rr.r());
                mk.drop(x, y, r, mix3(paper, base, dens), 1);
            } else {
                let r = cs[ci].r * (0.85 + 0.7 * rr.r());
                mk.drop(x, y, r, paper, 0);
            }
        }
        cs[ci].k += 1;
    }
    let b = p.num("breath");
    if b > 0.0 {
        let steps = 4 + round(6.0 * b) as i64;
        let freq = 0.7 + 0.9 * rr.r();
        let s = (rr.r() * 1e6).floor();
        for i in 0..steps {
            mk.breath(0.022 * b, freq, s + (i * 3) as f64, 2);
        }
        let fans = 1 + (rr.r() * 2.0).floor() as i64;
        for _ in 0..fans {
            let ax = rr.r();
            let ay = rr.r() * h;
            let a = rr.r() * TAU;
            let z = (0.06 + 0.08 * rr.r()) * b;
            mk.tine(ax, ay, cos(a), sin(a), z, 0.18);
        }
        if rr.r() < 0.6 {
            let sx = 0.2 + rr.r() * 0.6;
            let sy = h * (0.2 + rr.r() * 0.6);
            let sg = if rr.r() < 0.5 { -1.0 } else { 1.0 };
            let a = sg * (1.2 + 2.0 * rr.r()) * b;
            mk.swirl(sx, sy, a, 0.12);
        }
    }
}

fn render_sumi(seed: &str, w: usize, h: usize, p: &Params) -> Vec<u8> {
    let mut mk = Marble::new(seg_for(w) * 0.93);
    sumi_plan(seed, p, h as f64 / w as f64, &mut mk);
    let paper = sumi_paper();
    let mut img = vec![0.0f64; w * h * 3];
    for i in 0..w * h {
        img[i * 3] = paper[0];
        img[i * 3 + 1] = paper[1];
        img[i * 3 + 2] = paper[2];
    }
    let lw = jmax(0.5, w as f64 / 1600.0);
    draw_blobs(&mk, w, h, &mut img, &|b: &Blob| {
        if b.m != 0 {
            Some((b.c, 0.5, lw))
        } else {
            None
        }
    });
    let br = round(w as f64 / 1500.0) as i64;
    if br >= 1 {
        let mut ch = vec![vec![0.0f64; w * h]; 3];
        for i in 0..w * h {
            for c in 0..3 {
                ch[c][i] = img[i * 3 + c];
            }
        }
        let out: Vec<Vec<f64>> = ch.iter().map(|a| blur(a, w, h, br)).collect();
        for i in 0..w * h {
            for c in 0..3 {
                img[i * 3 + c] = out[c][i];
            }
        }
    }
    let sc = 700.0 / (w.max(h)) as f64;
    for y in 0..h {
        for x in 0..w {
            let f = fbm_s(x as f64 * sc * 0.02, y as f64 * sc * 0.02, 404.0, 4);
            let g = hash2(x as f64, y as f64, 77.0);
            let mv = clamp(jmax(0.0, f - 0.42) * 190.0 + g * 14.0, 0.0, 255.0) / 255.0;
            let i = (y * w + x) * 3;
            for c in 0..3 {
                let a = img[i + c];
                let s = 255.0 - ((255.0 - a) * (255.0 - mv * 255.0)) / 255.0;
                img[i + c] = a + (s - a) * 0.55;
            }
        }
    }
    let pt = paper_texture(w, h, "washi", 1.0);
    finish(&img, &pt, w, h)
}

pub const SUMINAGASHI: Generator = Generator {
    id: "suminagashi",
    title: "Suminagashi",
    description:
        "Japanese floating ink: alternating drops of ink and clear water make concentric rings that a breath distorts.",
    aspect: 1.5,
    params: &[
        choice(
            "ink",
            "Ink",
            "sumi",
            &[
                opt("sumi", "Sumi"),
                opt("ai", "Ai"),
                opt("sumi-ai", "Sumi + Ai"),
                opt("beni", "Beni"),
            ],
        ),
        number("rings", "Rings", 8.0, 70.0, 1.0, 34.0, ""),
        number("centers", "Centres (0 = auto)", 0.0, 6.0, 1.0, 0.0, ""),
        number("breath", "Breath", 0.0, 1.0, 0.01, 0.45, "%"),
    ],
    render: render_sumi,
};
