//! Mesh gradient: wide OKLab colour fields with folded silk-like sheets, a vortex and film grain.
//! Mirrors `mesh.ts`.

use super::core::*;
use super::spec::*;

fn pal(id: &str) -> &'static [&'static str] {
    match id {
        "sky" => &["#eef1f5", "#fbfcff", "#4f8fe6", "#f1ec97", "#a7c6f2", "#dcd6f2"],
        "sunset" => &[
            "#cfe6fb", "#3b30e0", "#9a45d6", "#e27aa2", "#f5d2a6", "#fbf6c0", "#c75a3e",
        ],
        "lagoon" => &["#1d8c90", "#0c2a44", "#d6f7a6", "#eef3b4", "#c99a1c", "#4cbfae"],
        "neon" => &["#26369a", "#a3288e", "#36c4e2", "#16246e", "#6b36aa", "#e04c9a"],
        _ => &["#f3f3f3", "#151515", "#8c8c8c", "#d6d6d6", "#454545"],
    }
}
const HUE: [f64; 6] = [0.0, 40.0, 180.0, 210.0, -35.0, 90.0];
const LL: [f64; 6] = [0.95, 0.55, 0.75, 0.35, 0.85, 0.65];
const CC: [f64; 6] = [0.03, 0.18, 0.14, 0.12, 0.08, 0.16];

struct Fold {
    arc: bool,
    cx: f64,
    cy: f64,
    r: f64,
    c: f64,
    th: f64,
    a: f64,
    f: f64,
    ph: f64,
    a2: f64,
    f2: f64,
    ph2: f64,
    side: f64,
    edge: V3,
    deep: V3,
    third: V3,
    depth: f64,
    sh: f64,
    off: f64,
    op: f64,
    fade: f64,
    gf: f64,
}

struct Pt {
    x: f64,
    y: f64,
    c: V3,
    s: f64,
}

fn render(seed: &str, w: usize, h: usize, p: &Params) -> Vec<u8> {
    let mut d = new_image(w, h);
    let mut rr = Rand::new(seed, "mesh");
    let s_ = (rr.r() * 1e6).floor();
    let asp = w as f64 / h as f64;
    let pal: Vec<V3> = if p.str("palette") == "random" {
        let hh0 = rr.r() * 360.0;
        let mut v = Vec::new();
        for i in 0..6 {
            let hh = hh0 + HUE[i] + (rr.r() - 0.5) * 20.0;
            v.push(oklch_fit(LL[i], CC[i], ((hh % 360.0) + 360.0) % 360.0));
        }
        v
    } else {
        pal(p.str("palette")).iter().map(|c| srgb_to_lab(hex_rgb(c))).collect()
    };
    let np = pal.len();
    let keys: Vec<f64> = (0..np).map(|_| rr.r()).collect();
    let mut order: Vec<usize> = (0..np).collect();
    order.sort_by(|&a, &b| {
        let k = keys[a] - keys[b];
        if k < 0.0 {
            std::cmp::Ordering::Less
        } else if k > 0.0 {
            std::cmp::Ordering::Greater
        } else {
            a.cmp(&b)
        }
    });
    let mut bp: Vec<Pt> = Vec::new();
    for i in 0..6 {
        let x = asp * (-0.15 + 1.3 * rr.r());
        let y = -0.15 + 1.3 * rr.r();
        let s = 0.28 + 0.3 * rr.r();
        bp.push(Pt {
            x,
            y,
            c: pal[order[i % np]],
            s,
        });
    }
    let k_n = round(p.num("folds")) as i64;
    let mut ls: Vec<Fold> = Vec::new();
    for _ in 0..k_n {
        let arc = rr.r() < 0.35;
        let t0 = (rr.r() - 0.5) * 1.6;
        let th = t0 + (if rr.r() < 0.5 { 0.0 } else { (PI / 2.0) * (rr.r() - 0.5) });
        let ei = (rr.r() * np as f64).floor() as usize;
        let mut di = (rr.r() * np as f64).floor() as usize;
        let mut t = 0;
        while t < 4 && di == ei {
            di = (rr.r() * np as f64).floor() as usize;
            t += 1;
        }
        let cx = asp * (-0.2 + 1.4 * rr.r());
        let cy = -0.2 + 1.4 * rr.r();
        let r = 0.35 + 0.6 * rr.r();
        let c = 0.2 + 0.6 * rr.r();
        let a = 0.08 + 0.22 * rr.r();
        let f = 1.2 + 2.2 * rr.r();
        let ph = rr.r() * TAU;
        let a2 = 0.03 + 0.06 * rr.r();
        let f2 = 3.0 + 3.0 * rr.r();
        let ph2 = rr.r() * TAU;
        let side = if rr.r() < 0.5 { 1.0 } else { -1.0 };
        let third = pal[(rr.r() * np as f64).floor() as usize];
        let depth = 0.12 + 0.35 * rr.r();
        let sh = 0.05 + 0.15 * rr.r();
        let off = rr.r() * 50.0;
        let op = 0.55 + 0.4 * rr.r();
        let fade = 0.15 + 0.35 * rr.r();
        let gf = 0.6 + 1.4 * rr.r();
        ls.push(Fold {
            arc,
            cx,
            cy,
            r,
            c,
            th,
            a,
            f,
            ph,
            a2,
            f2,
            ph2,
            side,
            edge: pal[ei],
            deep: pal[di],
            third,
            depth,
            sh,
            off,
            op,
            fade,
            gf,
        });
    }
    let vx = asp * (0.25 + 0.5 * rr.r());
    let vy = 0.25 + 0.5 * rr.r();
    let vsg = if rr.r() < 0.5 { 1.0 } else { -1.0 };
    let vs = vsg * (2.2 + 2.0 * rr.r()) * p.num("swirl");
    let vr = 0.18 + 0.25 * rr.r();
    let soft = p.num("soft");
    let gr = p.num("grain") * 0.11;
    let flow = p.num("flow");
    let hf = h as f64;
    for y in 0..h {
        let v0 = y as f64 / hf;
        for x in 0..w {
            let mut u = x as f64 / hf;
            let mut v = v0;
            u += (fbm2(u * 1.1 + 3.0, v * 1.1, s_, 3) - 0.5) * 0.35 * flow;
            v += (fbm2(u * 1.1, v * 1.1 + 7.0, s_ + 1.0, 3) - 0.5) * 0.35 * flow;
            {
                let dx = u - vx;
                let dy = v - vy;
                let ang = vs * exp(-(dx * dx + dy * dy) / (vr * vr));
                let ca = cos(ang);
                let sa = sin(ang);
                u = vx + dx * ca - dy * sa;
                v = vy + dx * sa + dy * ca;
            }
            let mut sw = 0.0;
            let mut c0 = 0.0;
            let mut c1 = 0.0;
            let mut c2 = 0.0;
            for q in &bp {
                let dx = u - q.x;
                let dy = v - q.y;
                let wt = exp(-(dx * dx + dy * dy) / (q.s * q.s));
                sw += wt;
                c0 += q.c[0] * wt;
                c1 += q.c[1] * wt;
                c2 += q.c[2] * wt;
            }
            c0 /= sw;
            c1 /= sw;
            c2 /= sw;
            for l in &ls {
                let sd;
                let along;
                if l.arc {
                    let dx = u - l.cx;
                    let dy = v - l.cy;
                    sd = (l.r - hypot2(dx, dy)) * l.side;
                    along = atan2(dy, dx) * l.r;
                } else {
                    let ct = cos(l.th);
                    let st = sin(l.th);
                    let uu = u * ct + v * st;
                    let vv = -u * st + v * ct;
                    let y0 = l.c + l.a * sin(uu * l.f + l.ph) + l.a2 * sin(uu * l.f2 + l.ph2);
                    let sl = l.a * l.f * cos(uu * l.f + l.ph) + l.a2 * l.f2 * cos(uu * l.f2 + l.ph2);
                    sd = ((vv - y0) / (1.0 + sl * sl).sqrt()) * l.side;
                    along = uu;
                }
                let sv = vnoise(along * 1.6 + l.off, 0.5, s_ + 7.0);
                let s = 0.004 + (0.03 + 0.38 * soft) * pow(sv, 1.2) + 0.03 * soft;
                let a = sstep(-s, s, sd);
                if sd < 0.0 {
                    c0 *= 1.0 - exp(sd / l.sh) * 0.15;
                }
                if a > 0.0 {
                    let t = sstep(0.0, l.depth, sd);
                    let g = (0.5 + 0.5 * sin(along * l.gf + l.off)) * 0.7;
                    let sdp = jmax(0.0, sd);
                    let hi = exp(-sdp / (s * 2.0 + 0.012)) * 0.07;
                    let m0 = l.edge[0] + (l.third[0] - l.edge[0]) * g;
                    let m1 = l.edge[1] + (l.third[1] - l.edge[1]) * g;
                    let m2 = l.edge[2] + (l.third[2] - l.edge[2]) * g;
                    let al = a * l.op * (0.3 + 0.7 * exp(-sdp / l.fade));
                    c0 += (m0 + (l.deep[0] - m0) * t + hi - c0) * al;
                    c1 += (m1 + (l.deep[1] - m1) * t - c1) * al;
                    c2 += (m2 + (l.deep[2] - m2) * t - c2) * al;
                }
            }
            let n = gr * ((hash2(x as f64, y as f64, s_) + hash2(x as f64, y as f64, s_ + 1.0) - 1.0) * 1.4);
            let rgb = oklab2lin(c0 + n * (0.6 + 0.4 * c0), c1 * 1.2, c2 * 1.2);
            put(&mut d, y * w + x, enc(rgb[0]), enc(rgb[1]), enc(rgb[2]));
        }
    }
    d
}

pub const MESH: Generator = Generator {
    id: "mesh-gradient",
    title: "Mesh gradient",
    description: "Soft gradients like blurred folded silk: OKLab colour fields, folds with crisp and misty edges, a vortex and film grain.",
    aspect: 1.5,
    params: &[
        choice(
            "palette",
            "Palette",
            "sunset",
            &[opt("sky", "Sky"), opt("sunset", "Sunset"), opt("lagoon", "Lagoon"), opt("neon", "Neon"), opt("mono", "Mono"), opt("random", "Random")],
        ),
        number("folds", "Folds", 0.0, 6.0, 1.0, 2.0, ""),
        number("soft", "Softness", 0.0, 1.0, 0.01, 0.65, "%"),
        number("swirl", "Swirl", 0.0, 1.0, 0.01, 0.45, "%"),
        number("flow", "Flow", 0.0, 1.0, 0.01, 0.5, "%"),
        number("grain", "Grain", 0.0, 1.0, 0.01, 0.4, "%"),
    ],
    render,
};
