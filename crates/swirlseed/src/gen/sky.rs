//! Sky: Rayleigh/Mie/ozone single scattering, volumetric clouds, cirrus, sun, moon and stars.
//! Mirrors `sky.ts`.

use super::core::*;
use super::spec::*;
use std::sync::OnceLock;

// ---------- gradient noise (per seed permutation) ----------
const GX: [f64; 8] = [1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 0.0, 0.0];
const GY: [f64; 8] = [1.0, 1.0, -1.0, -1.0, 0.0, 0.0, 1.0, -1.0];
struct GNoise {
    p: [u8; 512],
}
impl GNoise {
    fn new(r: &mut Rand) -> GNoise {
        let mut a = [0u8; 256];
        for (i, v) in a.iter_mut().enumerate() {
            *v = i as u8;
        }
        for i in (1..256usize).rev() {
            let j = (r.r() * (i + 1) as f64).floor() as usize;
            a.swap(i, j);
        }
        let mut p = [0u8; 512];
        for (i, v) in p.iter_mut().enumerate() {
            *v = a[i & 255];
        }
        GNoise { p }
    }
    fn n2(&self, x: f64, y: f64) -> f64 {
        let p = &self.p;
        let xi = x.floor();
        let yi = y.floor();
        let xf = x - xi;
        let yf = y - yi;
        let xx = (ti(xi) & 255) as usize;
        let yy = (ti(yi) & 255) as usize;
        let g00 = (p[p[xx] as usize + yy] & 7) as usize;
        let g01 = (p[p[xx] as usize + yy + 1] & 7) as usize;
        let g10 = (p[p[xx + 1] as usize + yy] & 7) as usize;
        let g11 = (p[p[xx + 1] as usize + yy + 1] & 7) as usize;
        let u = xf * xf * xf * (xf * (xf * 6.0 - 15.0) + 10.0);
        let v = yf * yf * yf * (yf * (yf * 6.0 - 15.0) + 10.0);
        let d00 = GX[g00] * xf + GY[g00] * yf;
        let d10 = GX[g10] * (xf - 1.0) + GY[g10] * yf;
        let d01 = GX[g01] * xf + GY[g01] * (yf - 1.0);
        let d11 = GX[g11] * (xf - 1.0) + GY[g11] * (yf - 1.0);
        let a0 = d00 + u * (d10 - d00);
        let b0 = d01 + u * (d11 - d01);
        a0 + v * (b0 - a0)
    }
    fn fbm(&self, mut x: f64, mut y: f64, oct: u32) -> f64 {
        let mut s = 0.0;
        let mut amp = 0.5;
        let mut tot = 0.0;
        for _ in 0..oct {
            s += amp * (self.n2(x, y) * 0.78 + 0.5);
            tot += amp;
            amp *= 0.5;
            let nx = (0.8 * x + 0.6 * y) * 2.03 + 17.3;
            y = (-0.6 * x + 0.8 * y) * 2.03 + 9.1;
            x = nx;
        }
        s / tot
    }
}

// ---------- physical atmosphere (km) ----------
const RE: f64 = 6360.0;
const RA: f64 = 6460.0;
const ESUN: f64 = 20.0;
const BR: [f64; 3] = [5.802e-3, 13.558e-3, 33.1e-3];
const BO: [f64; 3] = [0.65e-3, 1.881e-3, 0.085e-3];
#[allow(clippy::too_many_arguments)]
fn sph_t(px: f64, py: f64, pz: f64, dx: f64, dy: f64, dz: f64, r: f64) -> f64 {
    let b = px * dx + py * dy + pz * dz;
    let c = px * px + py * py + pz * pz - r * r;
    let d = b * b - c;
    if d < 0.0 {
        return -1.0;
    }
    let s = d.sqrt();
    let t1 = -b - s;
    let t2 = -b + s;
    if t1 > 0.0 {
        t1
    } else if t2 > 0.0 {
        t2
    } else {
        -1.0
    }
}
fn hits_ground(px: f64, py: f64, pz: f64, dx: f64, dy: f64, dz: f64) -> bool {
    let b = px * dx + py * dy + pz * dz;
    if b > 0.0 {
        return false;
    }
    let c = px * px + py * py + pz * pz - RE * RE;
    b * b - c > 0.0
}
#[allow(clippy::too_many_arguments)]
fn od_to(px: f64, py: f64, pz: f64, dx: f64, dy: f64, dz: f64, n: usize) -> V3 {
    let tm = sph_t(px, py, pz, dx, dy, dz, RA);
    let mut r = 0.0;
    let mut m = 0.0;
    let mut o = 0.0;
    if tm <= 0.0 {
        return [0.0, 0.0, 0.0];
    }
    let nf = n as f64;
    for i in 0..n {
        let a = i as f64 / nf;
        let b = (i + 1) as f64 / nf;
        let t = (tm * (a + b) * (a + b)) / 4.0;
        let dt = tm * (b * b - a * a);
        let x = px + dx * t;
        let y = py + dy * t;
        let z = pz + dz * t;
        let h = (x * x + y * y + z * z).sqrt() - RE;
        r += exp(-h / 8.0) * dt;
        m += exp(-h / 1.2) * dt;
        o += jmax(0.0, 1.0 - (h - 25.0).abs() / 15.0) * dt;
    }
    [r, m, o]
}
fn extinct(od: V3, m_k: f64) -> V3 {
    let me = 4.4e-3 * m_k;
    [
        exp(-(BR[0] * od[0] + me * od[1] + BO[0] * od[2])),
        exp(-(BR[1] * od[0] + me * od[1] + BO[1] * od[2])),
        exp(-(BR[2] * od[0] + me * od[1] + BO[2] * od[2])),
    ]
}
const NE: usize = 72;
const NA: usize = 40;
fn build_lut(light_el: f64, m_k: f64) -> Vec<f64> {
    let mut l = vec![0.0f64; NE * NA * 3];
    let sz = cos(light_el * DEG);
    let sy = sin(light_el * DEG);
    let oy = RE + 0.05;
    let g = 0.76;
    let ms = 3.996e-3 * m_k;
    let me = 4.4e-3 * m_k;
    for ie in 0..NE {
        let u = ie as f64 / (NE - 1) as f64;
        let el = jmax(0.4, 90.0 * u * u);
        let cv = cos(el * DEG);
        let vy = sin(el * DEG);
        for ia in 0..NA {
            let az = (ia as f64 / (NA - 1) as f64) * PI;
            let vx = cv * sin(az);
            let vz = cv * cos(az);
            let mu = vy * sy + vz * sz;
            let p_r = (3.0 / (16.0 * PI)) * (1.0 + mu * mu);
            let p_m = ((3.0 / (8.0 * PI)) * ((1.0 - g * g) * (1.0 + mu * mu)))
                / ((2.0 + g * g) * pow(1.0 + g * g - 2.0 * g * mu, 1.5));
            let tm = sph_t(0.0, oy, 0.0, vx, vy, vz, RA);
            let n = 22;
            let mut acc = [0.0f64; 3];
            let mut od_r = 0.0;
            let mut od_m = 0.0;
            let mut od_o = 0.0;
            for i in 0..n {
                let a = i as f64 / n as f64;
                let c = (i + 1) as f64 / n as f64;
                let t = (tm * (a + c) * (a + c)) / 4.0;
                let dt = tm * (c * c - a * a);
                let x = vx * t;
                let y = oy + vy * t;
                let z = vz * t;
                let h = (x * x + y * y + z * z).sqrt() - RE;
                let d_r = exp(-h / 8.0);
                let d_m = exp(-h / 1.2);
                let d_o = jmax(0.0, 1.0 - (h - 25.0).abs() / 15.0);
                od_r += d_r * dt * 0.5;
                od_m += d_m * dt * 0.5;
                od_o += d_o * dt * 0.5;
                if !hits_ground(x, y, z, 0.0, sy, sz) {
                    let ls = od_to(x, y, z, 0.0, sy, sz, 6);
                    for k in 0..3 {
                        let tau = BR[k] * (od_r + ls[0]) + me * (od_m + ls[1]) + BO[k] * (od_o + ls[2]);
                        acc[k] += (BR[k] * d_r * (p_r + 0.018) + ms * d_m * (p_m + 0.01)) * exp(-tau) * dt;
                    }
                }
                od_r += d_r * dt * 0.5;
                od_m += d_m * dt * 0.5;
                od_o += d_o * dt * 0.5;
            }
            let j = (ie * NA + ia) * 3;
            l[j] = acc[0] * ESUN;
            l[j + 1] = acc[1] * ESUN;
            l[j + 2] = acc[2] * ESUN;
        }
    }
    l
}
fn lut_at(l: &[f64], el: f64, d_az: f64) -> V3 {
    let u = (clamp(el, 0.0, 90.0) / 90.0).sqrt() * (NE - 1) as f64;
    let v = clamp(d_az / 180.0, 0.0, 1.0) * (NA - 1) as f64;
    let i0 = jmin((NE - 2) as f64, u.floor()) as usize;
    let j0 = jmin((NA - 2) as f64, v.floor()) as usize;
    let fu = u - i0 as f64;
    let fv = v - j0 as f64;
    let mut o = [0.0f64; 3];
    for k in 0..3 {
        let a = l[(i0 * NA + j0) * 3 + k];
        let b = l[(i0 * NA + j0 + 1) * 3 + k];
        let c = l[((i0 + 1) * NA + j0) * 3 + k];
        let d = l[((i0 + 1) * NA + j0 + 1) * 3 + k];
        o[k] = (a * (1.0 - fv) + b * fv) * (1.0 - fu) + (c * (1.0 - fv) + d * fv) * fu;
    }
    o
}
fn light_at(h: f64, el: f64, m_k: f64) -> V3 {
    let ce = cos(el * DEG);
    let sy = sin(el * DEG);
    let py = RE + h;
    if hits_ground(0.0, py, 0.0, 0.0, sy, ce) {
        return [0.0, 0.0, 0.0];
    }
    extinct(od_to(0.0, py, 0.0, 0.0, sy, ce, 10), m_k)
}

// ---------- baked tileable 3D noise for clouds (32³, seed-independent) ----------
fn h3(x: i64, y: i64, z: i64, s: i64) -> f64 {
    hash3(x as f64, y as f64, z as f64, s as f64)
}
fn wrap(i: i64, p: i64) -> i64 {
    ((i % p) + p) % p
}
fn vn_t(x: f64, y: f64, z: f64, p: i64, s: i64) -> f64 {
    let xi = x.floor();
    let yi = y.floor();
    let zi = z.floor();
    let xf = x - xi;
    let yf = y - yi;
    let zf = z - zi;
    let u = xf * xf * (3.0 - 2.0 * xf);
    let v = yf * yf * (3.0 - 2.0 * yf);
    let w = zf * zf * (3.0 - 2.0 * zf);
    let x0 = wrap(xi as i64, p);
    let y0 = wrap(yi as i64, p);
    let z0 = wrap(zi as i64, p);
    let x1 = (x0 + 1) % p;
    let y1 = (y0 + 1) % p;
    let z1 = (z0 + 1) % p;
    let l = |a: f64, b: f64, t: f64| a + (b - a) * t;
    l(
        l(
            l(h3(x0, y0, z0, s), h3(x1, y0, z0, s), u),
            l(h3(x0, y1, z0, s), h3(x1, y1, z0, s), u),
            v,
        ),
        l(
            l(h3(x0, y0, z1, s), h3(x1, y0, z1, s), u),
            l(h3(x0, y1, z1, s), h3(x1, y1, z1, s), u),
            v,
        ),
        w,
    )
}
fn wor_t(x: f64, y: f64, z: f64, p: i64, s: i64) -> f64 {
    let xi = x.floor() as i64;
    let yi = y.floor() as i64;
    let zi = z.floor() as i64;
    let mut m = 9.0;
    for k in -1..=1 {
        for j in -1..=1 {
            for i in -1..=1 {
                let cx = xi + i;
                let cy = yi + j;
                let cz = zi + k;
                let xx = wrap(cx, p);
                let yy = wrap(cy, p);
                let zz = wrap(cz, p);
                let dx = cx as f64 + h3(xx, yy, zz, s) - x;
                let dy = cy as f64 + h3(xx, yy, zz, s + 1) - y;
                let dz = cz as f64 + h3(xx, yy, zz, s + 2) - z;
                let d = dx * dx + dy * dy + dz * dz;
                if d < m {
                    m = d;
                }
            }
        }
    }
    jmin(1.0, m.sqrt())
}
fn equalise(a: &mut [f64]) {
    let mut h = vec![0.0f64; 2049];
    for v in a.iter() {
        h[jmin(2048.0, (v * 2048.0).floor()) as usize] += 1.0;
    }
    let mut c = 0.0;
    let n = a.len() as f64;
    for v in h.iter_mut() {
        c += *v;
        *v = c / n;
    }
    for v in a.iter_mut() {
        *v = h[jmin(2048.0, (*v * 2048.0).floor()) as usize];
    }
}
struct Tex {
    sh: Vec<f64>,
    de: Vec<f64>,
}
fn bake() -> &'static Tex {
    static TEX: OnceLock<Tex> = OnceLock::new();
    TEX.get_or_init(|| {
        const N: usize = 32;
        let mut sh = vec![0.0f64; N * N * N];
        let mut de = vec![0.0f64; N * N * N];
        let nf = N as f64;
        for z in 0..N {
            for y in 0..N {
                for x in 0..N {
                    let u = x as f64 / nf;
                    let v = y as f64 / nf;
                    let w = z as f64 / nf;
                    let mut pf = 0.0;
                    let mut a = 0.5;
                    let mut t = 0.0;
                    for o in 0..4i64 {
                        let f = 4i64 << o;
                        let ff = f as f64;
                        pf += a * vn_t(u * ff, v * ff, w * ff, f, 7 + o);
                        t += a;
                        a *= 0.5;
                    }
                    pf /= t;
                    let wf = 1.0
                        - (wor_t(u * 4.0, v * 4.0, w * 4.0, 4, 31) * 0.625
                            + wor_t(u * 8.0, v * 8.0, w * 8.0, 8, 37) * 0.25
                            + wor_t(u * 16.0, v * 16.0, w * 16.0, 16, 41) * 0.125);
                    sh[(z * N + y) * N + x] = clamp(
                        ((pf - (1.0 - wf)) / (1.0 - (1.0 - wf) * 0.98)) * 0.55 + wf * 0.55 - 0.1,
                        0.0,
                        1.0,
                    );
                    de[(z * N + y) * N + x] = 1.0
                        - (wor_t(u * 4.0, v * 4.0, w * 4.0, 4, 53) * 0.625
                            + wor_t(u * 8.0, v * 8.0, w * 8.0, 8, 59) * 0.25
                            + wor_t(u * 16.0, v * 16.0, w * 16.0, 16, 61) * 0.125);
                }
            }
        }
        equalise(&mut sh);
        equalise(&mut de);
        Tex { sh, de }
    })
}
fn tri(t: &[f64], x: f64, y: f64, z: f64) -> f64 {
    let xx = x.floor();
    let yy = y.floor();
    let zz = z.floor();
    let fx = x - xx;
    let fy = y - yy;
    let fz = z - zz;
    let xi = (ti(xx) & 31) as usize;
    let yi = (ti(yy) & 31) as usize;
    let zi = (ti(zz) & 31) as usize;
    let x1 = (xi + 1) & 31;
    let y1 = (yi + 1) & 31;
    let z1 = (zi + 1) & 31;
    let a = t[zi * 1024 + yi * 32 + xi];
    let b = t[zi * 1024 + yi * 32 + x1];
    let c = t[zi * 1024 + y1 * 32 + xi];
    let d = t[zi * 1024 + y1 * 32 + x1];
    let e = t[z1 * 1024 + yi * 32 + xi];
    let f = t[z1 * 1024 + yi * 32 + x1];
    let g = t[z1 * 1024 + y1 * 32 + xi];
    let h = t[z1 * 1024 + y1 * 32 + x1];
    let l0 = a + (b - a) * fx;
    let l1 = c + (d - c) * fx;
    let l2 = e + (f - e) * fx;
    let l3 = g + (h - g) * fx;
    let m0 = l0 + (l1 - l0) * fy;
    let m1 = l2 + (l3 - l2) * fy;
    m0 + (m1 - m0) * fz
}

struct Cloud {
    h0: f64,
    h1: f64,
    per: f64,
    sig: f64,
    ero: f64,
    sq: f64,
}
const CLOUD: [Option<Cloud>; 4] = [
    Some(Cloud {
        h0: 1.4,
        h1: 4.6,
        per: 17.0,
        sig: 34.0,
        ero: 0.36,
        sq: 1.0,
    }),
    Some(Cloud {
        h0: 1.2,
        h1: 2.3,
        per: 6.0,
        sig: 30.0,
        ero: 0.3,
        sq: 2.2,
    }),
    None,
    Some(Cloud {
        h0: 0.5,
        h1: 1.4,
        per: 9.0,
        sig: 7.0,
        ero: 0.45,
        sq: 2.2,
    }),
];
const TYPES: [&str; 4] = ["cumulus", "stratocumulus", "cirrus", "stratus"];
fn hg(g: f64, mu: f64) -> f64 {
    (1.0 - g * g) / (4.0 * PI * pow(1.0 + g * g - 2.0 * g * mu, 1.5))
}
fn sun_el_of(t: f64) -> f64 {
    52.0 * sin((2.0 * PI * (t - 6.0)) / 24.0)
}
fn or1(v: f64) -> f64 {
    if v == 0.0 || v.is_nan() {
        1.0
    } else {
        v
    }
}
fn light_tab(h: f64, tab: &[V3]) -> V3 {
    let f = clamp(h, 0.0, 11.99);
    let i = f.floor() as usize;
    let t = f - i as f64;
    let mut o = [0.0f64; 3];
    for k in 0..3 {
        o[k] = tab[i][k] + (tab[i + 1][k] - tab[i][k]) * t;
    }
    o
}

fn render(seed: &str, w: usize, h: usize, p: &Params) -> Vec<u8> {
    let nz = GNoise::new(&mut Rand::new(seed, "sky-noise"));
    let mut rr = Rand::new(seed, "sky");
    let ox = rr.r() * 500.0;
    let oy = rr.r() * 500.0;
    let wind = rr.r() * PI;
    let az0 = (rr.r() - 0.5) * 40.0;
    let moon_inc = (rr.r() - 0.5) * 14.0;
    let moon_az_o = (rr.r() - 0.5) * 16.0;
    let star_seed = (rr.r() * 1e9).floor();
    let mw_tilt = (rr.r() - 0.5) * 0.6;
    let ccx = rr.r() * 1000.0;
    let ccz = rr.r() * 1000.0;
    let shear = (rr.r() - 0.5) * 2.0;
    let time = p.num("time");
    let typ = TYPES.iter().position(|t| *t == p.str("type")).unwrap_or(0);
    let haze = p.num("haze");
    let tx = bake();
    let (wf, hf) = (w as f64, h as f64);
    let fovv = 62.0;
    let asp = wf / hf;
    let tan_v = sin((fovv / 2.0) * DEG) / cos((fovv / 2.0) * DEG);
    let tan_h = tan_v * asp;
    let sun_el = sun_el_of(time);
    let sun_az = az0 * 0.4 + (time - 12.0) * 5.5;
    let bottom_el = clamp(sun_el * 0.55 - 1.0, -0.8, 6.0);
    let pitch = (fovv / 2.0 + bottom_el) * DEG;
    let sin_p = sin(pitch);
    let cos_p = cos(pitch);
    let ce0 = cos(sun_el * DEG);
    let sx = ce0 * sin(sun_az * DEG);
    let sy = sin(sun_el * DEG);
    let sz = ce0 * cos(sun_az * DEG);
    let m_k = 1.0 + 3.5 * haze;
    let ls = build_lut(sun_el, m_k);
    let f_cyc = p.num("moon");
    let m_t = (((time - 24.0 * f_cyc) % 24.0) + 24.0) % 24.0;
    let moon_el = 52.0 * sin((2.0 * PI * (m_t - 6.0)) / 24.0) + moon_inc;
    let moon_az = az0 * 0.4 + moon_az_o + (m_t - 12.0) * 5.5;
    let mce = cos(moon_el * DEG);
    let mdx3 = mce * sin(moon_az * DEG);
    let mdy3 = sin(moon_el * DEG);
    let mdz3 = mce * cos(moon_az * DEG);
    let mzc = mdy3 * sin_p + mdz3 * cos_p;
    let myc = mdy3 * cos_p - mdz3 * sin_p;
    let moon_r = (0.26 / fovv) * hf * 2.2;
    let mut mx = -1e5;
    let mut my = -1e5;
    if mzc > 0.05 {
        mx = (0.5 + mdx3 / mzc / (2.0 * tan_h)) * wf;
        my = (0.5 - myc / mzc / (2.0 * tan_v)) * hf;
    }
    let i_ang = PI * (1.0 - 2.0 * f_cyc).abs();
    let frac_lit = (1.0 + cos(i_ang)) / 2.0;
    let (lx, ly, lz, mrot);
    {
        let dot = sx * mdx3 + sy * mdy3 + sz * mdz3;
        let mut px = sx - dot * mdx3;
        let mut py = sy - dot * mdy3;
        let mut pz = sz - dot * mdz3;
        let pl = or1((px * px + py * py + pz * pz).sqrt());
        px /= pl;
        py /= pl;
        pz /= pl;
        let hx = px;
        let hy = -(py * cos_p - pz * sin_p);
        let hl = or1((hx * hx + hy * hy).sqrt());
        let si = sin(i_ang);
        lx = (si * hx) / hl;
        ly = (si * hy) / hl;
        lz = cos(i_ang);
        mrot = atan2(-mdy3 * mdx3, (1.0 - mdy3 * mdy3) * cos_p + mdy3 * mdz3 * sin_p);
    }
    let mcr = cos(mrot);
    let msr = sin(mrot);
    let mw_a = 0.5 + mw_tilt;
    let moon_irr = if moon_el > -1.0 {
        3e-3 * pow(frac_lit, 1.5) * ESUN
    } else {
        0.0
    };
    let lm: Option<Vec<f64>> = if moon_irr > 0.0 {
        Some(build_lut(jmax(moon_el, 0.5), m_k))
    } else {
        None
    };
    let night_c: V3 = [0.9e-4 * ESUN, 1.4e-4 * ESUN, 2.6e-4 * ESUN];
    let mut lum = 0.0;
    for e in [15.0, 35.0, 60.0] {
        for a in [30.0, 90.0, 150.0] {
            let t = lut_at(&ls, e, a);
            lum += t[0] * 0.21 + t[1] * 0.72 + t[2] * 0.07;
            if let Some(lm) = &lm {
                let t = lut_at(lm, e, a);
                lum += ((t[0] * 0.21 + t[1] * 0.72 + t[2] * 0.07) * moon_irr) / ESUN;
            }
        }
    }
    lum = lum / 9.0 + night_c[1];
    let expo = clamp(0.16 / lum, 0.4, 0.016 / night_c[1]);
    let mut sun_l: Vec<V3> = Vec::new();
    let mut moon_l: Vec<V3> = Vec::new();
    for hh in 0..=12 {
        let s = light_at(hh as f64, sun_el, m_k);
        sun_l.push([s[0] * ESUN, s[1] * ESUN, s[2] * ESUN]);
        if moon_irr > 0.0 {
            let m = light_at(hh as f64, jmax(moon_el, 0.0), m_k);
            moon_l.push([m[0] * moon_irr, m[1] * moon_irr, m[2] * moon_irr]);
        } else {
            moon_l.push([0.0, 0.0, 0.0]);
        }
    }
    let mut amb = [0.0f64; 3];
    for e in [20.0, 50.0, 80.0] {
        for a in [0.0, 60.0, 120.0, 180.0] {
            let t = lut_at(&ls, e, a);
            for k in 0..3 {
                amb[k] += t[k] / 12.0;
            }
            if let Some(lm) = &lm {
                let t = lut_at(lm, e, a);
                for k in 0..3 {
                    amb[k] += (t[k] * moon_irr) / ESUN / 12.0;
                }
            }
        }
    }
    for k in 0..3 {
        amb[k] += night_c[k];
    }
    let use_moon = sun_el < -7.0 && moon_irr > 0.0;
    let ldx = if use_moon { mdx3 } else { sx };
    let ldy = if use_moon { mdy3 } else { sy };
    let ldz = if use_moon { mdz3 } else { sz };
    let lt: &[V3] = if use_moon { &moon_l } else { &sun_l };
    let cfg = &CLOUD[typ];
    let cov = if typ == 2 { 0.0 } else { p.num("cover") };
    let cir_amt = if typ == 2 {
        jmax(p.num("cover"), p.num("cirrus"))
    } else {
        p.num("cirrus")
    };
    let vis = 60.0 * (1.0 - 0.75 * haze);
    let cw = cos(wind);
    let swd = sin(wind);
    let shz = or1((sx * sx + sz * sz).sqrt());
    let mhz = or1((mdx3 * mdx3 + mdz3 * mdz3).sqrt());
    let sky_col = |dx: f64, dy: f64, dz: f64| -> V3 {
        let dyc = jmax(dy, 0.0105);
        let el = asin(dyc) / DEG;
        let hn = or1((dx * dx + dz * dz).sqrt());
        let mut o = lut_at(&ls, el, acos(clamp((dx * sx + dz * sz) / (hn * shz), -1.0, 1.0)) / DEG);
        o[0] += night_c[0];
        o[1] += night_c[1];
        o[2] += night_c[2];
        if let Some(lm) = &lm {
            let t2 = lut_at(
                lm,
                el,
                acos(clamp((dx * mdx3 + dz * mdz3) / (hn * mhz), -1.0, 1.0)) / DEG,
            );
            let k = moon_irr / ESUN;
            o[0] += t2[0] * k;
            o[1] += t2[1] * k;
            o[2] += t2[2] * k;
        }
        o
    };
    let ray = |fx: f64, fy: f64| -> V3 {
        let xc = (fx - 0.5) * 2.0 * tan_h;
        let yc = (0.5 - fy) * 2.0 * tan_v;
        let dyr = yc * cos_p + sin_p;
        let dzr = -yc * sin_p + cos_p;
        let n = 1.0 / (xc * xc + dyr * dyr + dzr * dzr).sqrt();
        [xc * n, dyr * n, dzr * n]
    };

    // ---- low clouds: volumetric ray march at half resolution ----
    let w2 = (wf / 2.0).ceil() as usize + 1;
    let h2 = (hf / 2.0).ceil() as usize + 1;
    let mut cb = vec![0.0f64; w2 * h2 * 4];
    let mut i = 3;
    while i < cb.len() {
        cb[i] = 1.0;
        i += 4;
    }
    if let (Some(cfg), true) = (cfg, cov > 0.005) {
        let q = if w >= 700 {
            1.0
        } else if w >= 400 {
            0.75
        } else {
            0.5
        };
        let ns = round(
            64.0 * q
                * (if typ == 3 {
                    0.4
                } else if typ == 1 {
                    0.75
                } else {
                    1.0
                }),
        ) as i64;
        let sig = cfg.sig;
        let sc = 32.0 / cfg.per;
        let dsc = 32.0 / 1.6;
        let thick = cfg.h1 - cfg.h0;
        let base: &[f64] = if q < 1.0 || typ == 3 {
            &[0.08, 0.3, 1.0]
        } else {
            &[0.05, 0.14, 0.32, 0.7, 1.4]
        };
        let lsteps: Vec<f64> = base.iter().map(|v| (v * thick) / 3.0).collect();
        let nl = lsteps.len();
        let ysc = if typ == 0 { 1.3 } else { 1.0 };
        let shape = |x: f64, y: f64, z: f64, wc: f64| -> f64 {
            let hfr = (y - cfg.h0) / thick;
            if hfr < 0.0 || hfr > 1.0 {
                return 0.0;
            }
            let prof = if typ == 0 {
                sstep(0.0, 0.07, hfr) * sstep(1.0, 0.45, hfr) * 0.6 + sstep(0.0, 0.07, hfr) * 0.4 * (1.0 - hfr)
            } else {
                sstep(0.0, 0.15, hfr) * sstep(1.0, 0.6, hfr)
            };
            let xs = x + ccx + hfr * shear;
            let zs = z + ccz;
            let rx = xs * cw + zs * swd;
            let rz = -xs * swd + zs * cw;
            let b1 = tri(&tx.sh, rx * sc, y * sc * ysc, (rz * sc) / cfg.sq) * 0.68;
            if (b1 + 0.32) * prof <= 1.0 - wc {
                return 0.0;
            }
            let b = b1
                + tri(
                    &tx.sh,
                    rx * sc * 3.7 + 17.0,
                    y * sc * 3.7,
                    (rz * sc * 3.7) / cfg.sq + 5.0,
                ) * 0.32;
            clamp((b * prof - (1.0 - wc)) / jmax(wc, 0.05), 0.0, 1.0)
        };
        for j in 0..h2 {
            for i in 0..w2 {
                let rd = ray(jmin(1.0, (i * 2) as f64 / wf), jmin(1.0, (j * 2) as f64 / hf));
                let (dx, dy, dz) = (rd[0], rd[1], rd[2]);
                let o = (j * w2 + i) * 4;
                if dy < 0.004 {
                    continue;
                }
                let oyy = RE + 0.02;
                let t0 = sph_t(0.0, oyy, 0.0, dx, dy, dz, RE + cfg.h0);
                let t1 = sph_t(0.0, oyy, 0.0, dx, dy, dz, RE + cfg.h1);
                if t0 < 0.0 || t0 > 140.0 {
                    continue;
                }
                let te = jmin(
                    t1,
                    t0 + (if typ == 0 {
                        16.0
                    } else if typ == 3 {
                        7.0
                    } else {
                        12.0
                    }),
                );
                let mu = dx * ldx + dy * ldy + dz * ldz;
                let ph0 = hg(0.75, mu) * 0.7 + hg(-0.25, mu) * 0.3;
                let ph1 = hg(0.4, mu) * 0.7 + hg(-0.15, mu) * 0.3;
                let ph2 = 1.0 / (4.0 * PI);
                let mut tr = 1.0;
                let mut r = 0.0;
                let mut g = 0.0;
                let mut b = 0.0;
                let mut tw = 0.0;
                let mut ww = 0.0;
                let mut miss = 0;
                let mut tau_c = -1.0;
                let mut hits: i64 = 0;
                let dt = (te - t0) / ns as f64;
                let mut t = t0 + dt * (0.25 + 0.5 * hash2(i as f64, j as f64, 77.0));
                let wc = clamp(
                    cov + (nz.fbm(
                        (dx * (t0 + te)) / 2.0 / 38.0 + ox,
                        (dz * (t0 + te)) / 2.0 / 38.0 + oy,
                        3,
                    ) - 0.5)
                        * 0.55
                        * (1.0 - cov * 0.6),
                    0.0,
                    1.0,
                );
                let mut s: i64 = 0;
                while s < ns && tr > 0.012 {
                    'body: {
                        let x = dx * t;
                        let z = dz * t;
                        let yy = oyy + dy * t;
                        let y = (x * x + yy * yy + z * z).sqrt() - RE;
                        let mut d = shape(x, y, z, wc);
                        if d <= 0.0 {
                            let m0 = miss;
                            miss += 1;
                            if m0 > 1 {
                                t += dt;
                                s += 1;
                            }
                            break 'body;
                        }
                        miss = 0;
                        let hfr = (y - cfg.h0) / thick;
                        let det = tri(&tx.de, x * dsc, y * dsc, z * dsc);
                        d = clamp((d - det * cfg.ero * (1.0 - d)) / (1.0 - det * cfg.ero * 0.5), 0.0, 1.0);
                        if d <= 0.002 {
                            break 'body;
                        }
                        let sg = d * sig;
                        let mut tau = tau_c;
                        let recompute = if tau < 0.0 {
                            true
                        } else {
                            let hv = hits;
                            hits += 1;
                            (hv & 1) == 0
                        };
                        if recompute {
                            tau = 0.0;
                            for k in 0..nl {
                                let l = lsteps[k];
                                let dl = if k > 0 { lsteps[k] - lsteps[k - 1] } else { lsteps[0] };
                                tau += shape(x + ldx * l, y + ldy * l, z + ldz * l, wc) * sig * dl;
                            }
                            tau_c = tau;
                        }
                        let pc = light_tab(y, lt);
                        let e = exp(-tau) * ph0 + 0.5 * exp(-tau * 0.25) * ph1 + 0.25 * exp(-tau * 0.06) * ph2;
                        let powder = 1.0 - exp(-sg * 0.35);
                        let lk = e * (0.5 + 0.5 * powder);
                        let ak = (0.25 + 0.75 * hfr) * (0.6 + 0.4 * powder) * 1.15;
                        let a = 1.0 - exp(-sg * dt);
                        r += tr * (pc[0] * lk + amb[0] * ak) * a;
                        g += tr * (pc[1] * lk + amb[1] * ak) * a;
                        b += tr * (pc[2] * lk + amb[2] * ak) * a;
                        tw += tr * a * t;
                        ww += tr * a;
                        tr *= 1.0 - a;
                    }
                    s += 1;
                    t += dt;
                }
                if ww > 0.0 {
                    let tmp = sky_col(dx, dy, dz);
                    let fog = exp(-tw / ww / vis);
                    let cvr = 1.0 - tr;
                    r = r * fog + tmp[0] * (1.0 - fog) * cvr;
                    g = g * fog + tmp[1] * (1.0 - fog) * cvr;
                    b = b * fog + tmp[2] * (1.0 - fog) * cvr;
                }
                cb[o] = r;
                cb[o + 1] = g;
                cb[o + 2] = b;
                cb[o + 3] = tr;
            }
        }
        let mut c2 = vec![0.0f64; cb.len()];
        let (w2i, h2i) = (w2 as i64, h2 as i64);
        for j in 0..h2i {
            for i in 0..w2i {
                for k in 0..4 {
                    let mut s = 0.0;
                    let mut wsum = 0.0;
                    for bb in -1i64..=1 {
                        for aa in -1i64..=1 {
                            let ii = i + aa;
                            let jj = j + bb;
                            if ii < 0 || jj < 0 || ii >= w2i || jj >= h2i {
                                continue;
                            }
                            let q2 = ((2 - aa.abs()) * (2 - bb.abs())) as f64;
                            s += cb[((jj * w2i + ii) * 4 + k) as usize] * q2;
                            wsum += q2;
                        }
                    }
                    c2[((j * w2i + i) * 4 + k) as usize] = s / wsum;
                }
            }
        }
        cb = c2;
    }

    // ---- sky and cirrus at half resolution ----
    let mut sb = vec![0.0f64; w2 * h2 * 3];
    let mut ct_ = vec![0.0f64; w2 * h2];
    let night = sstep(-4.0, -14.0, sun_el);
    for j in 0..h2 {
        for i in 0..w2 {
            let rd = ray(jmin(1.0, (i * 2) as f64 / wf), jmin(1.0, (j * 2) as f64 / hf));
            let (dx, dy, dz) = (rd[0], rd[1], rd[2]);
            let tmp = sky_col(dx, dy, dz);
            let mut rv = tmp[0];
            let mut gv = tmp[1];
            let mut bv = tmp[2];
            let el = asin(jmax(dy, 0.0105)) / DEG;
            let hgl = exp(-el / 6.0) * night;
            rv += hgl * night_c[0] * 1.6;
            gv += hgl * night_c[1] * 1.3;
            bv += hgl * night_c[2] * 0.9;
            let mut c_t = 1.0;
            if cir_amt > 0.02 && dy > 0.01 {
                let t = sph_t(0.0, RE + 0.02, 0.0, dx, dy, dz, RE + 9.0);
                if t > 0.0 && t < 400.0 {
                    let u = (dx * t) / 14.0;
                    let v = (dz * t) / 14.0;
                    let ru = u * cw + v * swd;
                    let rv2 = -u * swd + v * cw;
                    let qq = nz.fbm(ru * 0.35 + ox, rv2 * 0.9 + oy, 3);
                    let mut d = nz.fbm(ru * 0.3 + (qq - 0.5) * 1.5 + ox, rv2 * 1.3 + (qq - 0.5) * 0.5 + oy, 6);
                    d = clamp((d - (1.0 - 0.8 * cir_amt)) * 2.6, 0.0, 1.0)
                        * (0.6 + 0.4 * nz.fbm(ru * 3.0, rv2 * 7.0, 3));
                    let tau = d * 0.55 * jmin(1.0, cir_amt * 2.0);
                    if tau > 0.002 {
                        c_t = exp(-tau);
                        let mu = dx * ldx + dy * ldy + dz * ldz;
                        let t3 = light_tab(9.0, lt);
                        let ph = hg(0.7, mu) * 0.7 + hg(0.1, mu) * 0.3;
                        let a = 1.0 - c_t;
                        let fog = exp(-t / (vis * 3.0));
                        rv = rv * c_t + ((t3[0] * ph * 1.2 + amb[0] * 0.9) * fog + tmp[0] * (1.0 - fog)) * a;
                        gv = gv * c_t + ((t3[1] * ph * 1.2 + amb[1] * 0.9) * fog + tmp[1] * (1.0 - fog)) * a;
                        bv = bv * c_t + ((t3[2] * ph * 1.2 + amb[2] * 0.9) * fog + tmp[2] * (1.0 - fog)) * a;
                    }
                }
            }
            let o = j * w2 + i;
            sb[o * 3] = rv;
            sb[o * 3 + 1] = gv;
            sb[o * 3 + 2] = bv;
            ct_[o] = c_t;
        }
    }

    // ---- full resolution: sun, stars, moon, then composite ----
    const TMN: usize = 4096;
    let mut tml = vec![0.0f64; TMN + 1];
    for (i, v) in tml.iter_mut().enumerate() {
        let c = (i as f64 / TMN as f64) * (i as f64 / TMN as f64) * 24.0;
        let vv = (c * (2.51 * c + 0.03)) / (c * (2.43 * c + 0.59) + 0.14);
        *v = pow(clamp(vv, 0.0, 1.0), 1.0 / 2.2) * 255.0;
    }
    let tmg = |c: f64| -> f64 {
        let c = c * expo;
        if c <= 0.0 {
            return 0.0;
        }
        let f = (jmin(c, 24.0) / 24.0).sqrt() * TMN as f64;
        let i = f.floor() as usize;
        if i >= TMN {
            tml[TMN]
        } else {
            tml[i] + (tml[i + 1] - tml[i]) * (f - i as f64)
        }
    };
    let sun_r = 0.27 * DEG * 2.2;
    let cos_sun = cos(sun_r);
    let sun_disc = light_tab(0.0, &sun_l);
    let sun_rad = 1.0 / (PI * sun_r * sun_r);
    let mvis = extinct(od_to(0.0, RE + 0.05, 0.0, mdx3, jmax(mdy3, 0.01), mdz3, 8), m_k);
    let lmn = 0.03 * ESUN * 0.05;
    let mut d = new_image(w, h);
    for y in 0..h {
        let fy = y as f64 / hf;
        let yf = y as f64;
        for x in 0..w {
            let xf = x as f64;
            let fx = xf / wf;
            let rd = ray(fx, fy);
            let (dx, dy, dz) = (rd[0], rd[1], rd[2]);
            let dyc = jmax(dy, 0.0105);
            let mut b_r = 0.0;
            let mut b_g = 0.0;
            let mut b_b = 0.0;
            let cs = dx * sx + dyc * sy + dz * sz;
            if sun_el > -1.5 && cs > 0.95 {
                let ang = acos(clamp(cs, -1.0, 1.0));
                if cs > cos_sun - 0.00002 {
                    let q = ang / sun_r;
                    let ld = 0.4 + 0.6 * jmax(0.0, 1.0 - q * q).sqrt();
                    let cv = clamp((sun_r - ang) / (sun_r * 0.15), 0.0, 1.0) * ld * sun_rad;
                    b_r += sun_disc[0] * cv;
                    b_g += sun_disc[1] * cv;
                    b_b += sun_disc[2] * cv;
                }
                let gl = (0.9 * exp(-ang / (0.5 * DEG)) + 0.1 * exp(-ang / (2.5 * DEG))) * 0.12;
                b_r += sun_disc[0] * gl;
                b_g += sun_disc[1] * gl;
                b_b += sun_disc[2] * gl;
            }
            if night > 0.01 {
                let sk = (0.9 / expo) * night * sstep(0.0, 0.1, dy);
                let h1 = hash2(xf, yf, star_seed);
                let mut st = 0.0;
                let mut tint = 0.5;
                if h1 > 0.9988 {
                    st += pow((h1 - 0.9988) / 0.0012, 2.4) * 0.45 + 0.03;
                }
                let cx = (xf / 3.0).floor();
                let cy = (yf / 3.0).floor();
                let h2v = hash2(cx, cy, star_seed + 77.0);
                if h2v > 0.9855 {
                    let jx = hash2(cx, cy, 5.0) * 2.0 + 0.5;
                    let jy = hash2(cx, cy, 9.0) * 2.0 + 0.5;
                    let ddx = xf - cx * 3.0 - jx;
                    let ddy = yf - cy * 3.0 - jy;
                    st += (0.3 + ((h2v - 0.9855) / 0.0145) * 1.3) * exp(-(ddx * ddx + ddy * ddy) / 0.7);
                    tint = hash2(cx, cy, 3.0);
                }
                let wy = fy - (0.16 + mw_a * fx * 0.5 + 0.12);
                let wv =
                    exp(-(wy * wy) / 0.02) * jmax(0.0, 0.15 + 1.1 * (nz.fbm(fx * 6.0 + ox, fy * 6.0, 5) - 0.35)) * 0.08;
                let ex = exp(-0.25 / jmax(dyc, 0.03));
                b_r += (st * (0.8 + 0.4 * tint) + wv * 0.9) * sk * ex;
                b_g += (st * 0.9 + wv) * sk * ex;
                b_b += (st * (1.15 - 0.4 * tint) + wv * 1.3) * sk * ex;
            }
            if mx > -1e4 && moon_el > -1.0 {
                let mdx = xf - mx;
                let mdy = yf - my;
                let mr = (mdx * mdx + mdy * mdy).sqrt();
                if mr < moon_r * 12.0 {
                    let halo = (exp(-mr / (moon_r * 1.4)) * 0.02 + exp(-mr / (moon_r * 5.0)) * 0.006)
                        * pow(frac_lit, 1.6)
                        * lmn;
                    b_r += halo * mvis[0];
                    b_g += halo * mvis[1];
                    b_b += halo * mvis[2];
                    if mr < moon_r + 1.5 {
                        let c = clamp((moon_r + 0.7 - mr) / 1.4, 0.0, 1.0);
                        let nx = mdx / moon_r;
                        let ny = mdy / moon_r;
                        let nzz = jmax(0.0, 1.0 - nx * nx - ny * ny).sqrt();
                        let lit = sstep(-0.04, 0.06, nx * lx + ny * ly + nzz * lz);
                        let rx = nx * mcr - ny * msr;
                        let ry = nx * msr + ny * mcr;
                        let maria = 0.7 + 0.3 * sstep(0.35, 0.65, nz.fbm(rx * 2.2 + 4.0, ry * 2.2 + 9.0, 4));
                        let crat = 0.93 + 0.07 * nz.n2(rx * 14.0, ry * 14.0);
                        let bb = (lit * maria * crat * (0.9 + 0.2 * nzz) + 0.012 * (1.0 - lit)) * lmn * c;
                        b_r += bb * mvis[0];
                        b_g += bb * mvis[1];
                        b_b += bb * mvis[2];
                    }
                }
            }
            let gx = xf / 2.0;
            let gy = yf / 2.0;
            let i0 = gx.floor();
            let j0 = gy.floor();
            let ax = gx - i0;
            let ay = gy - j0;
            let (i0, j0) = (i0 as usize, j0 as usize);
            let mut rv = 0.0;
            let mut gv = 0.0;
            let mut bv = 0.0;
            let mut cr = 0.0;
            let mut cg = 0.0;
            let mut cbv = 0.0;
            let mut ct = 0.0;
            let mut c_t = 0.0;
            for k in 0..4usize {
                let ii = (i0 + (k & 1)).min(w2 - 1);
                let jj = (j0 + (k >> 1)).min(h2 - 1);
                let wt = (if (k & 1) != 0 { ax } else { 1.0 - ax }) * (if (k >> 1) != 0 { ay } else { 1.0 - ay });
                let q = jj * w2 + ii;
                cr += cb[q * 4] * wt;
                cg += cb[q * 4 + 1] * wt;
                cbv += cb[q * 4 + 2] * wt;
                ct += cb[q * 4 + 3] * wt;
                rv += sb[q * 3] * wt;
                gv += sb[q * 3 + 1] * wt;
                bv += sb[q * 3 + 2] * wt;
                c_t += ct_[q] * wt;
            }
            let ct3 = ct * ct * ct * c_t;
            rv = rv * ct + b_r * ct3 + cr;
            gv = gv * ct + b_g * ct3 + cg;
            bv = bv * ct + b_b * ct3 + cbv;
            let dn = hash2(xf, yf, 991.0) - 0.5;
            put(&mut d, y * w + x, tmg(rv) + dn, tmg(gv) + dn, tmg(bv) + dn);
        }
    }
    d
}

pub const SKY: Generator = Generator {
    id: "sky",
    title: "Sky",
    description: "A physically scattered sky at any hour: volumetric clouds, cirrus, haze, sun, a moving moon with phases, and stars.",
    aspect: 1.6,
    params: &[
        number("time", "Time", 0.0, 24.0, 0.05, 17.6, "h"),
        choice(
            "type",
            "Clouds",
            "cumulus",
            &[opt("cumulus", "Cumulus"), opt("stratocumulus", "Stratocumulus"), opt("cirrus", "Cirrus"), opt("stratus", "Stratus")],
        ),
        number("cover", "Cover", 0.0, 1.0, 0.01, 0.42, "%"),
        number("cirrus", "Cirrus veil", 0.0, 1.0, 0.01, 0.12, "%"),
        number("haze", "Haze", 0.0, 1.0, 0.01, 0.2, "%"),
        number("moon", "Moon phase", 0.0, 1.0, 0.01, 0.38, ""),
    ],
    render,
};
