//! Metaballs: graphic 2D looks and a ray-marched 3D implicit surface on a studio floor.
//! Mirrors `metaballs.ts`.

use super::core::*;
use super::spec::*;

struct Lava {
    bg: [[f64; 3]; 2],
    wax: [f64; 3],
}
const LAVA: [Lava; 4] = [
    Lava {
        bg: [[64.0, 10.0, 70.0], [200.0, 40.0, 70.0]],
        wax: [255.0, 150.0, 40.0],
    },
    Lava {
        bg: [[10.0, 30.0, 90.0], [40.0, 120.0, 170.0]],
        wax: [150.0, 255.0, 90.0],
    },
    Lava {
        bg: [[90.0, 10.0, 20.0], [210.0, 70.0, 30.0]],
        wax: [255.0, 220.0, 80.0],
    },
    Lava {
        bg: [[20.0, 10.0, 60.0], [90.0, 40.0, 150.0]],
        wax: [255.0, 90.0, 160.0],
    },
];
const GLOSS: [[[f64; 3]; 3]; 4] = [
    [[255.0, 92.0, 92.0], [255.0, 180.0, 60.0], [255.0, 120.0, 190.0]],
    [[90.0, 150.0, 255.0], [120.0, 230.0, 220.0], [170.0, 120.0, 255.0]],
    [[255.0, 210.0, 80.0], [255.0, 130.0, 70.0], [250.0, 240.0, 200.0]],
    [[120.0, 220.0, 140.0], [60.0, 170.0, 200.0], [240.0, 250.0, 140.0]],
];
const VIV_HEX: [&str; 8] = [
    "#ec2f55", "#ee7419", "#7b3fb4", "#2f6fe4", "#c9318d", "#f6466f", "#9b4fd6", "#ff8a3d",
];
const LOOKS2: [&str; 5] = ["layers", "gradient", "drops", "contour", "holo"];

fn aces(c: f64) -> f64 {
    let v = (c * (2.51 * c + 0.03)) / (c * (2.43 * c + 0.59) + 0.14);
    enc(if v < 0.0 {
        0.0
    } else if v > 1.0 {
        1.0
    } else {
        v
    })
}
fn film(th: f64) -> V3 {
    [
        0.5 + 0.5 * cos(TAU * th),
        0.5 + 0.5 * cos(TAU * (th + 0.33)),
        0.5 + 0.5 * cos(TAU * (th + 0.67)),
    ]
}
fn h3(v: f64) -> f64 {
    // `hypot3(...) || 1`
    if v == 0.0 || v.is_nan() {
        1.0
    } else {
        v
    }
}

// ---------------- 2D ----------------
#[derive(Clone, Copy)]
struct Ball2 {
    x: f64,
    y: f64,
    r: f64,
    c: usize,
}
struct Group {
    b: Vec<Ball2>,
    bb: [f64; 4],
    c1: V3,
    c2: V3,
    ang: f64,
    al: f64,
}
fn bbox(g: &mut Group) {
    let mut x0 = 1e9;
    let mut y0 = 1e9;
    let mut x1 = -1e9;
    let mut y1 = -1e9;
    for q in &g.b {
        x0 = jmin(x0, q.x - q.r);
        y0 = jmin(y0, q.y - q.r);
        x1 = jmax(x1, q.x + q.r);
        y1 = jmax(y1, q.y + q.r);
    }
    g.bb = [x0, y0, x1, y1];
}
fn new_group() -> Group {
    Group {
        b: Vec::new(),
        bb: [0.0; 4],
        c1: [0.0; 3],
        c2: [0.0; 3],
        ang: 0.0,
        al: 1.0,
    }
}
fn layout_2d(rr: &mut Rand, n: usize, size: f64, asp: f64) -> Vec<Ball2> {
    let mut bs: Vec<Ball2> = Vec::new();
    let nc = jmax(1.0, round(n as f64 / 3.2)) as usize;
    let mut cs: Vec<(f64, f64)> = Vec::new();
    for _ in 0..nc {
        let mut bx = 0.0;
        let mut by = 0.0;
        let mut bd = -1.0;
        for _ in 0..14 {
            let x = asp * (0.12 + 0.76 * rr.r());
            let y = 0.15 + 0.7 * rr.r();
            let mut m = 9.0;
            for q in &cs {
                m = jmin(m, hypot2(x - q.0, y - q.1));
            }
            if m > bd {
                bd = m;
                bx = x;
                by = y;
            }
        }
        cs.push((bx, by));
    }
    for i in 0..n {
        let c = i % nc;
        let r = size * (0.06 + 0.12 * pow(rr.r(), 1.3)) * (if i < nc { 1.35 } else { 1.0 });
        let mut x;
        let mut y;
        let mates: Vec<Ball2> = bs.iter().filter(|b| b.c == c).copied().collect();
        if mates.is_empty() {
            x = cs[c].0;
            y = cs[c].1;
        } else {
            let p = mates[(rr.r() * mates.len() as f64).floor() as usize];
            let a = rr.r() * TAU;
            let dd = (p.r / 2.05 + r) * (0.95 + 0.45 * rr.r());
            x = p.x + cos(a) * dd;
            y = p.y + sin(a) * dd;
        }
        x = clamp(x, r * 0.6, asp - r * 0.6);
        y = clamp(y, r * 0.6, 1.0 - r * 0.6);
        bs.push(Ball2 { x, y, r: r * 2.05, c });
    }
    bs
}
fn field_balls(bs: &[Ball2], u: f64, v: f64) -> (f64, f64, f64) {
    let mut f = 0.0;
    let mut fx = 0.0;
    let mut fy = 0.0;
    for b in bs {
        let dx = u - b.x;
        let dy = v - b.y;
        let r2 = b.r * b.r;
        let q = (dx * dx + dy * dy) / r2;
        if q >= 1.0 {
            continue;
        }
        let oo = 1.0 - q;
        f += oo * oo * oo;
        let gd = (-6.0 * oo * oo) / r2;
        fx += gd * dx;
        fy += gd * dy;
    }
    (f, fx, fy)
}

#[allow(clippy::too_many_arguments)]
fn add_group(rr: &mut Rand, groups: &mut Vec<Group>, cx: f64, cy: f64, n: usize, r0: f64, spread: f64) {
    let mut g = new_group();
    for i in 0..n {
        let r = r0 * (0.55 + 0.7 * rr.r());
        let mut x = cx;
        let mut y = cy;
        if i > 0 {
            let q = g.b[(rr.r() * g.b.len() as f64).floor() as usize];
            let a = rr.r() * TAU;
            let dd = (q.r / 2.05 + r) * (spread * (0.6 + 0.6 * rr.r()));
            x = q.x + cos(a) * dd;
            y = q.y + sin(a) * dd;
        }
        g.b.push(Ball2 {
            x,
            y,
            r: r * 2.05,
            c: 0,
        });
    }
    bbox(&mut g);
    groups.push(g);
}

fn render_2d(seed: &str, w: usize, h: usize, p: &Params, look: &str) -> Vec<u8> {
    let mut d = new_image(w, h);
    let mut rr = Rand::new(seed, "metaballs-2d");
    let s_ = (rr.r() * 1e6).floor();
    let (wf, hf) = (w as f64, h as f64);
    let asp = wf / hf;
    let t_ = 0.42 - 0.3 * p.num("merge");
    let n = round(p.num("balls")) as usize;
    let sz = p.num("size") / 0.5;
    let ang = p.num("light") * DEG;
    let lx = sin(ang) * 0.7;
    let ly = -0.7;
    let viv: Vec<V3> = VIV_HEX.iter().map(|h| lin3(hex_rgb(h))).collect();
    let mut groups: Vec<Group> = Vec::new();
    if look == "layers" {
        let nb = jmax(2.0, round(n as f64 / 4.0)) as usize;
        for _ in 0..nb {
            let cx = asp * (0.12 + 0.76 * rr.r());
            let cy = 0.12 + 0.76 * rr.r();
            let cnt = 4 + (rr.r() * 5.0).floor() as usize;
            add_group(&mut rr, &mut groups, cx, cy, cnt, 0.1 * sz, 1.05);
        }
        let ns = round(n as f64 * 1.6) as usize;
        for _ in 0..ns {
            let cx = asp * rr.r();
            let cy = rr.r();
            let r0 = 0.02 * sz + 0.045 * sz * pow(rr.r(), 1.5);
            add_group(&mut rr, &mut groups, cx, cy, 1, r0, 1.0);
        }
    } else if look == "gradient" {
        let mut g = new_group();
        let mut x = -0.04;
        let mut y = 0.3 + 0.4 * rr.r();
        let mut a = (rr.r() - 0.5) * 1.4;
        let nn = clamp(round(n as f64 * 0.45), 4.0, 9.0) as usize;
        let stp = (asp * 0.95) / nn as f64;
        for i in 0..nn {
            let r = (0.08 + 0.09 * rr.r()) * sz * (if i == nn - 1 { 1.5 } else { 1.0 });
            g.b.push(Ball2 {
                x,
                y,
                r: r * 2.05,
                c: 0,
            });
            a += (rr.r() - 0.5) * 2.2;
            if y < 0.3 {
                a = a.abs() * 0.8 + 0.25;
            }
            if y > 0.7 {
                a = -a.abs() * 0.8 - 0.25;
            }
            a = clamp(a, -0.85, 0.85);
            x += cos(a) * stp;
            y += sin(a) * stp * 0.9;
        }
        bbox(&mut g);
        groups.push(g);
    } else if look == "drops" {
        let mut g = new_group();
        for _ in 0..n {
            let r = (0.035 + 0.06 * pow(rr.r(), 1.3)) * sz;
            let mut x = 0.0;
            let mut y = 0.0;
            let mut ok = false;
            let mut t = 0;
            while t < 30 && !ok {
                x = asp * 0.5 + (rr.r() - 0.5) * asp * 0.62;
                y = 0.5 + (rr.r() - 0.5) * 0.68;
                ok = true;
                for q in &g.b {
                    let dq = hypot2(q.x - x, q.y - y);
                    let lim = (q.r / 2.05 + r) * (if rr.r() < 0.2 { 0.9 } else { 1.6 });
                    if !(dq > lim) {
                        ok = false;
                        break;
                    }
                }
                t += 1;
            }
            g.b.push(Ball2 {
                x,
                y,
                r: r * 2.05,
                c: 0,
            });
        }
        bbox(&mut g);
        groups.push(g);
    } else {
        let mut g = new_group();
        g.b = layout_2d(&mut rr, n, p.num("size"), asp);
        bbox(&mut g);
        groups.push(g);
    }
    let dark = rr.r() < 0.55;
    let bg_a = lin3(if dark { [24.0, 24.0, 28.0] } else { [118.0, 10.0, 72.0] });
    let bg_b = lin3(if dark { [20.0, 20.0, 24.0] } else { [106.0, 16.0, 170.0] });
    for g in groups.iter_mut() {
        g.c1 = viv[(rr.r() * viv.len() as f64).floor() as usize];
        g.c2 = viv[(rr.r() * viv.len() as f64).floor() as usize];
        g.ang = rr.r() * TAU;
        g.al = if g.b.len() > 1 {
            0.55 + 0.2 * rr.r()
        } else {
            0.7 + 0.25 * rr.r()
        };
    }
    let all: Vec<Ball2> = groups.iter().flat_map(|g| g.b.iter().copied()).collect();
    let grad_a = lin3([66.0, 84.0, 240.0]);
    let grad_b = lin3([196.0, 40.0, 220.0]);
    let grad_c = lin3([246.0, 24.0, 118.0]);
    let paper = lin3([241.0, 238.0, 230.0]);
    let ink = lin3([28.0, 28.0, 34.0]);
    let fo = rr.r() * 10.0;
    let lv: Vec<f64> = [0.12, 0.2, 0.3, 0.45, 0.65, 0.85, 1.2, 1.7, 2.4, 3.4]
        .iter()
        .map(|k| t_ * k)
        .collect();
    let bg_d0 = lin3([232.0, 246.0, 234.0]);
    let bg_d1 = lin3([214.0, 224.0, 236.0]);
    let mut gc = [0.0f64; 3];
    for y in 0..h {
        let v = y as f64 / hf;
        for x in 0..w {
            let u = x as f64 / hf;
            let mut r;
            let mut g;
            let mut b;
            if look == "layers" {
                r = lerp(bg_a[0], bg_b[0], v);
                g = lerp(bg_a[1], bg_b[1], v);
                b = lerp(bg_a[2], bg_b[2], v);
                for gg in &groups {
                    if u < gg.bb[0] || u > gg.bb[2] || v < gg.bb[1] || v > gg.bb[3] {
                        continue;
                    }
                    let o = field_balls(&gg.b, u, v);
                    if o.0 < t_ * 0.5 {
                        continue;
                    }
                    let gl = hypot2(o.1, o.2) + 1e-9;
                    let cov = clamp(((o.0 - t_) / gl) * hf + 0.5, 0.0, 1.0);
                    if cov <= 0.0 {
                        continue;
                    }
                    let bx = gg.bb;
                    let t = clamp(
                        ((u - (bx[0] + bx[2]) / 2.0) * cos(gg.ang) + (v - (bx[1] + bx[3]) / 2.0) * sin(gg.ang))
                            / jmax(bx[2] - bx[0], bx[3] - bx[1])
                            + 0.5,
                        0.0,
                        1.0,
                    );
                    let a = cov * gg.al;
                    r = lerp(r, lerp(gg.c1[0], gg.c2[0], t), a);
                    g = lerp(g, lerp(gg.c1[1], gg.c2[1], t), a);
                    b = lerp(b, lerp(gg.c1[2], gg.c2[2], t), a);
                }
            } else if look == "gradient" {
                r = 0.79;
                g = 0.79;
                b = 0.8;
                let gg = &groups[0];
                if !(u < gg.bb[0] || u > gg.bb[2] || v < gg.bb[1] || v > gg.bb[3]) {
                    let o = field_balls(&gg.b, u, v);
                    let gl = hypot2(o.1, o.2) + 1e-9;
                    let cov = clamp(((o.0 - t_) / gl) * hf + 0.5, 0.0, 1.0);
                    if cov > 0.0 {
                        let t = clamp((v - gg.bb[1]) / (gg.bb[3] - gg.bb[1]), 0.0, 1.0);
                        for c in 0..3 {
                            gc[c] = if t < 0.5 {
                                lerp(grad_a[c], grad_b[c], t * 2.0)
                            } else {
                                lerp(grad_b[c], grad_c[c], t * 2.0 - 1.0)
                            };
                        }
                        r = lerp(r, gc[0], cov);
                        g = lerp(g, gc[1], cov);
                        b = lerp(b, gc[2], cov);
                    }
                }
            } else if look == "drops" {
                let t = clamp((u / asp) * 0.6 + v * 0.4, 0.0, 1.0);
                r = lerp(bg_d0[0], bg_d1[0], t);
                g = lerp(bg_d0[1], bg_d1[1], t);
                b = lerp(bg_d0[2], bg_d1[2], t);
                let o2 = field_balls(&all, u - lx * 0.012 * sz, v - ly * 0.012 * sz);
                let sh = sstep(t_ * 0.3, t_ * 1.3, o2.0);
                let o = field_balls(&all, u, v);
                let f = o.0;
                let gl = hypot2(o.1, o.2) + 1e-9;
                let cov = clamp(((f - t_) / gl) * hf + 0.5, 0.0, 1.0);
                let k = 1.0 - 0.07 * sh * (1.0 - cov);
                r *= k;
                g *= k;
                b *= k;
                if cov > 0.0 {
                    let sq = 2.0 * jmax(f - t_, 1e-6).sqrt();
                    let nx = (-o.1 * 0.09 * sz) / sq;
                    let ny = (-o.2 * 0.09 * sz) / sq;
                    let nl = hypot3(nx, ny, 1.0);
                    let ex = 1.0 - 1.0 / nl;
                    let base = [r, g, b];
                    let mut rrr = [0.0f64; 3];
                    for c in 0..3 {
                        let o_ = (c as f64 - 1.0) * 0.05;
                        let dk = 0.62 * sstep(0.45 + o_, 0.9 + o_, ex) * (1.0 - 0.45 * sstep(0.9 + o_, 1.0, ex));
                        rrr[c] = base[c] * (1.0 - dk);
                    }
                    let lit = (nx * lx + ny * ly) / nl;
                    let q1 = (ex - 0.12) / 0.05;
                    let q2 = (ex - 0.35) / 0.25;
                    let hl = pow(clamp(lit * 1.6, 0.0, 1.0), 6.0) * sstep(0.3, 0.7, ex) * 0.55
                        + exp(-q1 * q1) * clamp(-lit, 0.0, 1.0) * 0.12;
                    let ca = exp(-q2 * q2) * clamp(-lit * 2.0, 0.0, 1.0) * 0.08;
                    r = lerp(r, rrr[0] + hl + ca, cov);
                    g = lerp(g, rrr[1] + hl + ca, cov);
                    b = lerp(b, rrr[2] + hl + ca, cov);
                }
            } else if look == "contour" {
                let o = field_balls(&all, u, v);
                let f = o.0;
                let gl = hypot2(o.1, o.2) + 1e-9;
                let cov = clamp(((f - t_) / gl) * hf + 0.5, 0.0, 1.0);
                r = paper[0] - cov * 0.06;
                g = paper[1] - cov * 0.06;
                b = paper[2] - cov * 0.048;
                for &l in &lv {
                    let dpx = ((f - l).abs() / gl) * hf;
                    let main = if l == lv[3] { 0.0 } else { 1.0 };
                    let wd = if l == t_ { 1.3 } else { 0.55 };
                    let a = clamp(wd - dpx + 0.5, 0.0, 1.0) * (if l == t_ { 1.0 } else { 0.75 }) * (main + 1.0 - main);
                    r = lerp(r, ink[0], a);
                    g = lerp(g, ink[1], a);
                    b = lerp(b, ink[2], a);
                }
            } else {
                let o = field_balls(&all, u, v);
                let f = o.0;
                let gl = hypot2(o.1, o.2) + 1e-9;
                let cov = clamp(((f - t_) / gl) * hf + 0.5, 0.0, 1.0);
                let fc = film(fo + (f / t_) * 0.55);
                let gw = clamp(f / t_, 0.0, 1.0) * 0.35 * (1.0 - cov);
                let kk = 0.75 + 0.25 * sin((f / t_) * 9.0);
                r = lerp(0.012 + fc[0] * gw * 0.4, fc[0] * kk, cov);
                g = lerp(0.012 + fc[1] * gw * 0.4, fc[1] * kk, cov);
                b = lerp(0.02 + fc[2] * gw * 0.4, fc[2] * kk, cov);
            }
            let gr = if look == "gradient" {
                0.0
            } else {
                (hash2(x as f64, y as f64, s_) - 0.5) * 0.012
            };
            put(&mut d, y * w + x, enc(r + gr), enc(g + gr), enc(b + gr));
        }
    }
    d
}

// ---------------- 3D ----------------
#[allow(clippy::too_many_arguments)]
fn bx(az: f64, el: f64, a0: f64, a1: f64, e0: f64, e1: f64, k: f64) -> f64 {
    sstep(a0 - 0.06, a0, az) * sstep(a1 + 0.06, a1, az) * sstep(e0 - 0.04, e0, el) * sstep(e1 + 0.04, e1, el) * k
}
fn env3(dx: f64, dy: f64, dz: f64, warm: bool) -> V3 {
    let el = dy;
    let az = atan2(dx, dz);
    let c = if el > 0.0 {
        let mut c = lerp(0.05, 0.22, pow(el, 0.7));
        c += bx(az, el, -1.3, -0.5, 0.2, 0.7, 6.0)
            + bx(az, el, 0.5, 1.1, 0.12, 0.5, 3.4)
            + bx(az, el, -0.4, 0.4, 0.82, 1.2, 5.0)
            + bx(az, el, 2.2, 2.9, 0.05, 0.45, 2.6);
        c
    } else {
        lerp(0.1, 0.03, jmin(1.0, -el * 3.0))
    };
    if warm {
        [c * 1.04, c, c * 0.94]
    } else {
        [c * 0.96, c, c * 1.05]
    }
}
#[derive(Clone, Copy)]
struct Ball3 {
    x: f64,
    y: f64,
    z: f64,
    rr: f64,
    r: f64,
    r2: f64,
    c: usize,
}

struct Scene3 {
    b: Vec<Ball3>,
    gs: Vec<f64>,
    t: f64,
    lx: f64,
    ly: f64,
    lz: f64,
    glass: bool,
    lst: Vec<usize>,
    tn: usize,
    fo: [f64; 7],
}
impl Scene3 {
    fn field(&mut self, px: f64, py: f64, pz: f64, n: usize) {
        let mut f = 0.0;
        let mut gx = 0.0;
        let mut gy = 0.0;
        let mut gz = 0.0;
        let mut w0 = 0.0;
        let mut w1 = 0.0;
        let mut w2 = 0.0;
        for k in 0..n {
            let b = &self.b[self.lst[k]];
            let dx = px - b.x;
            let dy = py - b.y;
            let dz = pz - b.z;
            let q = (dx * dx + dy * dy + dz * dz) / b.r2;
            if q >= 1.0 {
                continue;
            }
            let o = 1.0 - q;
            let g = o * o * o;
            let gd = (-6.0 * o * o) / b.r2;
            f += g;
            gx += gd * dx;
            gy += gd * dy;
            gz += gd * dz;
            let c = b.c % 3;
            if c == 0 {
                w0 += g;
            } else if c == 1 {
                w1 += g;
            } else {
                w2 += g;
            }
        }
        self.fo = [f, gx, gy, gz, w0, w1, w2];
    }
    #[allow(clippy::too_many_arguments)]
    fn trace(&mut self, ox: f64, oy: f64, oz: f64, dx: f64, dy: f64, dz: f64, t_max: f64, skip: f64) -> f64 {
        let mut n = 0;
        let mut t0 = 1e9;
        let mut t1 = -1.0;
        for i in 0..self.b.len() {
            let b = &self.b[i];
            let cx = b.x - ox;
            let cy = b.y - oy;
            let cz = b.z - oz;
            let tc = cx * dx + cy * dy + cz * dz;
            let d2 = cx * cx + cy * cy + cz * cz - tc * tc;
            if d2 >= b.r2 {
                continue;
            }
            let h = (b.r2 - d2).sqrt();
            let a = tc - h;
            let e = tc + h;
            if e <= 0.001 {
                continue;
            }
            self.lst[n] = i;
            n += 1;
            if a < t0 {
                t0 = a;
            }
            if e > t1 {
                t1 = e;
            }
        }
        if n == 0 {
            return -1.0;
        }
        let mut t = jmax(t0, if skip > 0.0 { skip } else { 0.001 });
        let mut g = 0.0;
        let mut max_step = 1e9;
        for k in 0..n {
            g += self.gs[self.lst[k]];
            max_step = jmin(max_step, self.b[self.lst[k]].rr);
        }
        max_step *= 0.35;
        t1 = jmin(t1, t_max);
        let mut prev = t;
        let mut s = 0;
        while s < 600 && t < t1 {
            self.field(ox + dx * t, oy + dy * t, oz + dz * t, n);
            if self.fo[0] >= self.t {
                let mut a = prev;
                let mut b = t;
                for _ in 0..7 {
                    let m = (a + b) / 2.0;
                    self.field(ox + dx * m, oy + dy * m, oz + dz * m, n);
                    if self.fo[0] >= self.t {
                        b = m;
                    } else {
                        a = m;
                    }
                }
                self.tn = n;
                return b;
            }
            prev = t;
            t += clamp(((self.t - self.fo[0]) / g) * 0.9, 0.004, max_step);
            s += 1;
        }
        -1.0
    }
    fn shadow(&self, px: f64, py: f64, pz: f64, selfs: bool) -> (f64, f64) {
        let mut s = 1.0;
        let mut ao = 1.0;
        for b in &self.b {
            let re = b.rr * 1.15;
            let cx = b.x - px;
            let cy = b.y - py;
            let cz = b.z - pz;
            let dist = hypot3(cx, cy, cz);
            if selfs && dist < re * 1.25 {
                continue;
            }
            let tc = cx * self.lx + cy * self.ly + cz * self.lz;
            if tc > 0.0 {
                let dp = jmax(0.0, dist * dist - tc * tc).sqrt();
                let k = clamp((dp - re) / (tc * 0.32 + 0.02) + 0.5, 0.0, 1.0);
                s *= if self.glass { 1.0 - 0.45 * (1.0 - k) } else { k };
            }
            let q = re / jmax(dist, 1e-3);
            ao *= 1.0 - 0.75 * jmin(1.0, q * q) * clamp((cy / dist) * 0.5 + 0.6, 0.0, 1.0);
        }
        (s, ao)
    }
}

struct Ctx<'a> {
    sc: Scene3,
    look: &'a str,
    lava: bool,
    glass: bool,
    warm: bool,
    floor_c: V3,
    s_: f64,
    fo: f64,
    gp: [V3; 3],
    bg0: V3,
    bg1: V3,
    wax: V3,
    ex0: f64,
    ey0: f64,
    ez0: f64,
    fov: f64,
    fx: f64,
    fy: f64,
    fz: f64,
    rx: f64,
    rz: f64,
    ux: f64,
    uy: f64,
    uz: f64,
    asp: f64,
    w: f64,
    h: f64,
}
impl<'a> Ctx<'a> {
    fn gloss_like(&self) -> bool {
        self.look == "glossy" || self.glass
    }
    fn wall(&self, dy: f64) -> V3 {
        let k = 1.0 - 0.5 * sstep(0.0, 0.45, dy);
        let f = self.floor_c;
        if self.gloss_like() {
            [f[0] * 0.95 * k, f[1] * 0.95 * k, f[2] * 0.95 * k]
        } else {
            [f[0] * 1.8 * k, f[1] * 1.8 * k, f[2] * 2.0 * k]
        }
    }
    fn floor_at(&self, qx: f64, qz: f64, dist: f64) -> V3 {
        let sh = self.sc.shadow(qx, 0.0, qz, false);
        let lit = (0.22 + 0.78 * sh.0 * self.sc.ly) * sh.1;
        let fade = sstep(5.0, 22.0, dist);
        let bgc = self.wall(0.0);
        let f = self.floor_c;
        [
            lerp(f[0] * lit * 2.2, bgc[0], fade),
            lerp(f[1] * lit * 2.2, bgc[1], fade),
            lerp(f[2] * lit * 2.2, bgc[2], fade),
        ]
    }
    fn normal(&self) -> V3 {
        let o = &self.sc.fo;
        let l = h3(hypot3(o[1], o[2], o[3]));
        [-o[1] / l, -o[2] / l, -o[3] / l]
    }
    #[allow(clippy::too_many_arguments)]
    fn reflect_col(&mut self, px: f64, py: f64, pz: f64, dx: f64, dy: f64, dz: f64, depth: i32) -> V3 {
        let th = if depth < 1 {
            self.sc.trace(px, py, pz, dx, dy, dz, 30.0, 0.03)
        } else {
            -1.0
        };
        let tf = if dy < 0.0 { -py / dy } else { 1e9 };
        if th > 0.0 && th < tf {
            let tn = self.sc.tn;
            self.sc.field(px + dx * th, py + dy * th, pz + dz * th, tn);
            let [nx, ny, nz] = self.normal();
            let dn = dx * nx + dy * ny + dz * nz;
            let e = env3(dx - 2.0 * dn * nx, dy - 2.0 * dn * ny, dz - 2.0 * dn * nz, self.warm);
            return [e[0] * 0.7, e[1] * 0.7, e[2] * 0.7];
        }
        if tf < 1e8 {
            return self.floor_at(px + dx * tf, pz + dz * tf, hypot3(dx * tf, dy * tf, dz * tf));
        }
        env3(dx, dy, dz, self.warm)
    }
    fn tint(&self, hx: f64, hy: f64, hz: f64, extra: f64) -> V3 {
        let f = film(self.fo + hy * 0.35 + fbm2(hx * 0.6, hz * 0.6, self.s_ + 9.0, 3) * 1.4 + extra);
        [0.15 + f[0] * 1.05, 0.15 + f[1] * 1.05, 0.15 + f[2] * 1.05]
    }
    fn shade(&mut self, sx: f64, sy: f64) -> [f64; 4] {
        let (ww, hh) = (self.w, self.h);
        let px = ((sx / ww) * 2.0 - 1.0) * self.fov * self.asp;
        let py = (1.0 - (sy / hh) * 2.0) * self.fov;
        let mut dx = self.fx + self.rx * px + self.ux * py;
        let mut dy = self.fy + self.uy * py;
        let mut dz = self.fz + self.rz * px + self.uz * py;
        {
            let l = hypot3(dx, dy, dz);
            dx /= l;
            dy /= l;
            dz /= l;
        }
        let (ex0, ey0, ez0) = (self.ex0, self.ey0, self.ez0);
        let (lx, ly, lz) = (self.sc.lx, self.sc.ly, self.sc.lz);
        let t_ = self.sc.t;
        let th = self.sc.trace(ex0, ey0, ez0, dx, dy, dz, 60.0, 0.0);
        if th < 0.0 {
            if self.lava {
                let v = sy / hh;
                let u = sx / ww;
                let k = 0.75 + 0.5 * fbm2(u * 2.0 + 3.0, v * 1.2, self.s_ + 3.0, 4);
                let gq = (v - 1.05) * 2.2;
                let glow = exp(-gq * gq) * 0.9;
                let vg = 1.0 - 0.55 * pow((u - 0.5).abs() * 2.0, 2.4);
                let (b0, b1, wx) = (self.bg0, self.bg1, self.wax);
                return [
                    (lerp(b1[0], b0[0], v) * k + wx[0] * glow * 0.5) * vg,
                    (lerp(b1[1], b0[1], v) * k + wx[1] * glow * 0.35) * vg,
                    (lerp(b1[2], b0[2], v) * k + wx[2] * glow * 0.2) * vg,
                    0.0,
                ];
            }
            if dy < 0.0 {
                let tf = -ey0 / dy;
                let qx = ex0 + dx * tf;
                let qz = ez0 + dz * tf;
                let mut c = self.floor_at(qx, qz, tf);
                if !self.gloss_like() {
                    let fr = 0.06 + 0.5 * pow(1.0 + dy, 5.0);
                    let rc = self.sc.trace(qx, 0.001, qz, dx, -dy, dz, 30.0, 0.0);
                    if rc > 0.0 {
                        let hx = qx + dx * rc;
                        let hy = -dy * rc;
                        let hz = qz + dz * rc;
                        let tn = self.sc.tn;
                        self.sc.field(hx, hy, hz, tn);
                        let [nx, ny, nz] = self.normal();
                        let dn = dx * nx - dy * ny + dz * nz;
                        let e = env3(dx - 2.0 * dn * nx, -dy - 2.0 * dn * ny, dz - 2.0 * dn * nz, self.warm);
                        let k: V3 = if self.look == "iridescent" {
                            self.tint(hx, hy, hz, 0.4)
                        } else {
                            [1.0, 1.0, 1.0]
                        };
                        let fade = exp(-hy * 1.1) * (fr * 0.6 + 0.12);
                        c[0] += e[0] * k[0] * fade;
                        c[1] += e[1] * k[1] * fade;
                        c[2] += e[2] * k[2] * fade;
                    }
                }
                return [c[0], c[1], c[2], 0.0];
            }
            let wl = self.wall(dy);
            return [wl[0], wl[1], wl[2], 0.0];
        }
        let mut out = [0.0f64; 4];
        out[3] = 1.0;
        let hx = ex0 + dx * th;
        let hy = ey0 + dy * th;
        let hz = ez0 + dz * th;
        let tn = self.sc.tn;
        self.sc.field(hx, hy, hz, tn);
        let w0 = self.sc.fo[4];
        let w1 = self.sc.fo[5];
        let w2 = self.sc.fo[6];
        let [nx, ny, nz] = self.normal();
        let ndv = jmax(0.0, -(dx * nx + dy * ny + dz * nz));
        let fr = 0.04 + 0.96 * pow(1.0 - ndv, 5.0);
        let dn = dx * nx + dy * ny + dz * nz;
        let rdx = dx - 2.0 * dn * nx;
        let rdy = dy - 2.0 * dn * ny;
        let rdz = dz - 2.0 * dn * nz;
        let ndl = nx * lx + ny * ly + nz * lz;
        let hvx = lx - dx;
        let hvy = ly - dy;
        let hvz = lz - dz;
        let ndh = jmax(0.0, (nx * hvx + ny * hvy + nz * hvz) / hypot3(hvx, hvy, hvz));
        self.sc.field(hx + nx * 0.12, hy + ny * 0.12, hz + nz * 0.12, tn);
        let ao1 = clamp(1.0 - (self.sc.fo[0] - t_ * 0.5) * 1.6, 0.25, 1.0);
        let look = self.look;
        if look == "chrome" || look == "iridescent" {
            let k = 0.78 + 0.22 * fr;
            let tt: V3 = if look == "iridescent" {
                self.tint(hx, hy, hz, (1.0 - ndv) * 0.9)
            } else {
                [1.0, 1.0, 1.0]
            };
            let sh = self.sc.shadow(hx, hy, hz, true);
            let sp = pow(ndh, 500.0) * 30.0 * sh.0;
            let e = self.reflect_col(hx + nx * 0.005, hy + ny * 0.005, hz + nz * 0.005, rdx, rdy, rdz, 0);
            out[0] = e[0] * k * tt[0] * ao1 + sp;
            out[1] = e[1] * k * tt[1] * ao1 + sp;
            out[2] = e[2] * k * tt[2] * ao1 + sp;
        } else if self.glass {
            let eta = 1.0 / 1.45;
            let c_i = -dn;
            let k1 = jmax(0.0, 1.0 - eta * eta * (1.0 - c_i * c_i)).sqrt();
            let tx = eta * dx + (eta * c_i - k1) * nx;
            let ty = eta * dy + (eta * c_i - k1) * ny;
            let tz = eta * dz + (eta * c_i - k1) * nz;
            let mut t2 = 0.02;
            let mut qx = hx;
            let mut qy = hy;
            let mut qz = hz;
            for _ in 0..80 {
                t2 += 0.025;
                qx = hx + tx * t2;
                qy = hy + ty * t2;
                qz = hz + tz * t2;
                self.sc.field(qx, qy, qz, tn);
                if self.sc.fo[0] < t_ {
                    break;
                }
            }
            self.sc.field(qx, qy, qz, tn);
            let fo = self.sc.fo;
            let ml = h3(hypot3(fo[1], fo[2], fo[3]));
            let mx = fo[1] / ml;
            let my = fo[2] / ml;
            let mz = fo[3] / ml;
            let c2 = -(tx * mx + ty * my + tz * mz);
            let eta2 = 1.45;
            let k2 = 1.0 - eta2 * eta2 * (1.0 - c2 * c2);
            let (ox, oy, oz);
            if k2 < 0.0 {
                let dd = tx * mx + ty * my + tz * mz;
                ox = tx - 2.0 * dd * mx;
                oy = ty - 2.0 * dd * my;
                oz = tz - 2.0 * dd * mz;
            } else {
                let s2 = k2.sqrt();
                ox = eta2 * tx - (eta2 * c2 - s2) * mx;
                oy = eta2 * ty - (eta2 * c2 - s2) * my;
                oz = eta2 * tz - (eta2 * c2 - s2) * mz;
            }
            let e = env3(rdx, rdy, rdz, true);
            let fk = 0.04 + 0.96 * pow(1.0 - ndv, 5.0);
            let sp = pow(ndh, 400.0) * 25.0;
            for c in 0..3 {
                let s = 1.0 + (c as f64 - 1.0) * 0.06;
                let ax = dx + (ox - dx) * s;
                let ay = dy + (oy - dy) * s;
                let az = dz + (oz - dz) * s;
                let al = hypot3(ax, ay, az);
                let bc = self.reflect_col(qx, qy, qz, ax / al, ay / al, az / al, 1);
                out[c] = bc[c] * 0.93 * (1.0 - fk) + e[c] * fk * 1.4 + sp;
            }
        } else if look == "glossy" {
            let ws = w0 + w1 + w2 + 1e-9;
            let sh = self.sc.shadow(hx, hy, hz, true);
            let s = sh.0;
            let dif = jmax(0.0, ndl) * s * 1.25;
            let amb = (0.32 + 0.22 * ny) * ao1 * sh.1;
            let wrap = (jmax(0.0, ndl + 0.35) / 1.35) * 0.18;
            let e = self.reflect_col(hx + nx * 0.005, hy + ny * 0.005, hz + nz * 0.005, rdx, rdy, rdz, 1);
            let cc = 0.04 + 0.96 * pow(1.0 - ndv, 5.0);
            let sp = pow(ndh, 180.0) * 9.0 * s + pow(ndh, 28.0) * 0.3 * s;
            let gp = self.gp;
            for c in 0..3 {
                let al = (gp[0][c] * w0 + gp[1][c] * w1 + gp[2][c] * w2) / ws;
                out[c] = al * (dif + amb + wrap) + e[c] * cc * 0.9 + sp;
            }
        } else {
            let mut t2 = th + 0.01;
            let mut thick = 0.0;
            for _ in 0..40 {
                t2 += 0.04;
                self.sc.field(ex0 + dx * t2, ey0 + dy * t2, ez0 + dz * t2, tn);
                if self.sc.fo[0] < t_ {
                    break;
                }
                thick += 0.04;
            }
            let thin = exp(-thick * 2.2);
            let below = clamp(0.45 - 0.7 * ny, 0.0, 1.2);
            let core = 1.0 - thin;
            let sat = 0.55 + 0.45 * core;
            let glow = (0.45 + 1.1 * below) * (0.7 + 0.9 * thin) + 0.25 * jmax(0.0, ndl);
            let rim = pow(1.0 - ndv, 3.0) * 0.6;
            let sp = pow(ndh, 70.0) * 1.2 + fr * 0.25;
            let wx = self.wax;
            out[0] = wx[0] * glow + wx[0] * rim + sp;
            out[1] = wx[1] * glow * (1.0 - 0.35 * core * sat) + wx[1] * rim * 0.8 + sp;
            out[2] = wx[2] * glow * (1.0 - 0.6 * core * sat) + wx[2] * rim * 0.6 + sp;
        }
        out
    }
}

fn render_3d(seed: &str, w: usize, h: usize, p: &Params, look: &str) -> Vec<u8> {
    let mut rr = Rand::new(seed, "metaballs-3d");
    let s_ = (rr.r() * 1e6).floor();
    let lava = look == "lava";
    let glass = look == "glass";
    let n = round(p.num("balls")) as usize;
    let size = p.num("size");
    let mut bs: Vec<Ball3> = Vec::new();
    let nc = jmax(1.0, round(n as f64 / 3.2)) as usize;
    let mut cs: Vec<(f64, f64)> = Vec::new();
    for _ in 0..nc {
        let mut bx_ = 0.0;
        let mut bz = 0.0;
        let mut bd = -1.0;
        for _ in 0..12 {
            let x = -1.9 + 3.8 * rr.r();
            let z = -1.6 + 2.6 * rr.r();
            let mut m = 9.0;
            for q in &cs {
                m = jmin(m, hypot2(x - q.0, z - q.1));
            }
            if m > bd {
                bd = m;
                bx_ = x;
                bz = z;
            }
        }
        cs.push((bx_, bz));
    }
    for i in 0..n {
        let c = i % nc;
        let r = size * (0.32 + 0.5 * pow(rr.r(), 1.3)) * (if i < nc { 1.3 } else { 1.0 });
        let mates: Vec<Ball3> = bs.iter().filter(|b| b.c == c).copied().collect();
        let (x, y, z);
        if mates.is_empty() {
            x = cs[c].0;
            z = cs[c].1;
            y = if lava { 0.4 + 2.6 * rr.r() } else { r * 0.9 };
        } else {
            let q = mates[(rr.r() * mates.len() as f64).floor() as usize];
            let a = rr.r() * TAU;
            let e = (rr.r() - 0.3) * 1.2;
            let dd = (q.rr + r) * (0.85 + 0.3 * rr.r());
            x = q.x + cos(a) * cos(e) * dd;
            z = q.z + sin(a) * cos(e) * dd * 0.7;
            y = jmax(if lava { 0.2 } else { r * 0.85 }, q.y + sin(e) * dd);
        }
        bs.push(Ball3 {
            x,
            y,
            z,
            rr: r,
            r: r * 2.05,
            r2: r * r * 4.2025,
            c,
        });
    }
    let gs: Vec<f64> = bs.iter().map(|b| 1.9 / b.r).collect();
    let t_ = 0.42 - 0.3 * p.num("merge");
    let ang = p.num("light") * DEG;
    let (lx, ly, lz) = {
        let mut lx = sin(ang) * 0.7;
        let mut ly = 0.65;
        let mut lz = cos(ang) * 0.45 + 0.2;
        let l = hypot3(lx, ly, lz);
        lx /= l;
        ly /= l;
        lz /= l;
        (lx, ly, lz)
    };
    let sc = Scene3 {
        b: bs,
        gs,
        t: t_,
        lx,
        ly,
        lz,
        glass,
        lst: vec![0; n.max(1)],
        tn: 0,
        fo: [0.0; 7],
    };
    let lv = &LAVA[(rr.r() * LAVA.len() as f64).floor() as usize];
    let g0 = GLOSS[(rr.r() * GLOSS.len() as f64).floor() as usize];
    let gp = [lin3(g0[0]), lin3(g0[1]), lin3(g0[2])];
    let bg0 = lin3(lv.bg[0]);
    let bg1 = lin3(lv.bg[1]);
    let wax = lin3(lv.wax);
    let fo = rr.r() * 10.0;
    let ex0 = 0.0;
    let ey0 = if lava { 1.7 } else { 1.9 };
    let ez0 = if lava { 7.2 } else { 6.0 };
    let fa = ((if lava { 34.0 } else { 30.0 }) / 2.0) * DEG;
    let fov = sin(fa) / cos(fa);
    let mut fx = -ex0;
    let mut fy = (if lava { 1.5 } else { 0.6 }) - ey0;
    let mut fz = -ez0;
    {
        let l = hypot3(fx, fy, fz);
        fx /= l;
        fy /= l;
        fz /= l;
    }
    let mut rx = -fz;
    let mut rz = fx;
    {
        let l = hypot2(rx, rz);
        rx /= l;
        rz /= l;
    }
    let ux = -rz * fy;
    let uy = rz * fx - rx * fz;
    let uz = rx * fy;
    let (wf, hf) = (w as f64, h as f64);
    let floor_c: V3 = if look == "glossy" || glass {
        lin3(if glass {
            [214.0, 224.0, 232.0]
        } else {
            [236.0, 230.0, 222.0]
        })
    } else if look == "chrome" {
        [0.05, 0.05, 0.055]
    } else {
        [0.03, 0.03, 0.045]
    };
    let mut cx = Ctx {
        sc,
        look,
        lava,
        glass,
        warm: look != "iridescent",
        floor_c,
        s_,
        fo,
        gp,
        bg0,
        bg1,
        wax,
        ex0,
        ey0,
        ez0,
        fov,
        fx,
        fy,
        fz,
        rx,
        rz,
        ux,
        uy,
        uz,
        asp: wf / hf,
        w: wf,
        h: hf,
    };
    let mut hit = vec![0u8; w * h];
    let mut buf = vec![0.0f64; w * h * 3];
    for y in 0..h {
        for x in 0..w {
            let o = cx.shade(x as f64 + 0.5, y as f64 + 0.5);
            let i = y * w + x;
            buf[i * 3] = o[0];
            buf[i * 3 + 1] = o[1];
            buf[i * 3 + 2] = o[2];
            hit[i] = o[3] as u8;
        }
    }
    let offs = [0.25, 0.25, 0.75, 0.25, 0.25, 0.75, 0.75, 0.75];
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let i = y * w + x;
            let hh = hit[i];
            if hit[i - 1] == hh && hit[i + 1] == hh && hit[i - w] == hh && hit[i + w] == hh {
                continue;
            }
            let mut r = 0.0;
            let mut g = 0.0;
            let mut b = 0.0;
            let mut k = 0;
            while k < 8 {
                let o = cx.shade(x as f64 + offs[k], y as f64 + offs[k + 1]);
                r += o[0];
                g += o[1];
                b += o[2];
                k += 2;
            }
            buf[i * 3] = r / 4.0;
            buf[i * 3 + 1] = g / 4.0;
            buf[i * 3 + 2] = b / 4.0;
        }
    }
    let ex = if lava {
        1.05
    } else if look == "glossy" {
        1.15
    } else if glass {
        1.05
    } else {
        1.5
    };
    const GS: [f64; 9] = [0.15, 0.03, 0.05, 0.2, 0.008, 0.045, 0.88, 0.06, 0.02];
    let mut d = new_image(w, h);
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let gr = (hash2(x as f64, y as f64, s_) - 0.5) * 0.004;
            let mut r = aces(buf[i * 3] * ex + gr);
            let mut g = aces(buf[i * 3 + 1] * ex + gr);
            let mut b = aces(buf[i * 3 + 2] * ex + gr);
            if lava {
                let u = (x as f64 + 0.5) / wf;
                let mut a = 0.0;
                let mut k = 0;
                while k < 9 {
                    let t = (u - GS[k]).abs() / (GS[k + 1] * 2.0);
                    if t < 1.0 {
                        a += GS[k + 2] * (1.0 - t);
                    }
                    k += 3;
                }
                r += 255.0 * a;
                g += 255.0 * a;
                b += 255.0 * a;
            }
            put(&mut d, i, r, g, b);
        }
    }
    d
}

fn render(seed: &str, w: usize, h: usize, p: &Params) -> Vec<u8> {
    let look = p.str("look");
    if LOOKS2.contains(&look) {
        render_2d(seed, w, h, p, look)
    } else {
        render_3d(seed, w, h, p, look)
    }
}

pub const METABALLS: Generator = Generator {
    id: "metaballs",
    title: "Metaballs",
    description:
        "Blobs that melt together: ray-marched 3D chrome, glass, plastic, lava and oil film, or 2D graphic looks.",
    aspect: 1.5,
    params: &[
        choice(
            "look",
            "Look",
            "chrome",
            &[
                opt_g("chrome", "Chrome", "3D"),
                opt_g("glass", "Glass", "3D"),
                opt_g("glossy", "Glossy", "3D"),
                opt_g("lava", "Lava", "3D"),
                opt_g("iridescent", "Iridescent", "3D"),
                opt_g("layers", "Layers", "2D"),
                opt_g("gradient", "Gradient", "2D"),
                opt_g("drops", "Drops", "2D"),
                opt_g("contour", "Contour", "2D"),
                opt_g("holo", "Holo", "2D"),
            ],
        ),
        number("balls", "Blobs", 3.0, 22.0, 1.0, 10.0, ""),
        number("size", "Size", 0.25, 0.8, 0.01, 0.5, "%"),
        number("merge", "Merge", 0.0, 1.0, 0.01, 0.55, "%"),
        number("light", "Light", -80.0, 80.0, 1.0, -30.0, "°"),
    ],
    render,
};
