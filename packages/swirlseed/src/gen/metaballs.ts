/** Metaballs: graphic 2D looks and a ray-marched 3D implicit surface on a studio floor. */
import { DEG, TAU, atan2, clamp, cos, enc, exp, fbm2, hash2, hexRgb, hypot2, hypot3, lerp, lin3, newImage, pow, put, round, sin, sstep, stream, type Rand, type V3 } from "./base.js";
import { type Generator, type ParamValues, num, opt, str } from "./spec.js";

const LAVA: { bg: [V3, V3]; wax: V3 }[] = [
  { bg: [[64, 10, 70], [200, 40, 70]], wax: [255, 150, 40] },
  { bg: [[10, 30, 90], [40, 120, 170]], wax: [150, 255, 90] },
  { bg: [[90, 10, 20], [210, 70, 30]], wax: [255, 220, 80] },
  { bg: [[20, 10, 60], [90, 40, 150]], wax: [255, 90, 160] },
];
const GLOSS: V3[][] = [
  [[255, 92, 92], [255, 180, 60], [255, 120, 190]],
  [[90, 150, 255], [120, 230, 220], [170, 120, 255]],
  [[255, 210, 80], [255, 130, 70], [250, 240, 200]],
  [[120, 220, 140], [60, 170, 200], [240, 250, 140]],
];
const VIV: V3[] = ["#ec2f55", "#ee7419", "#7b3fb4", "#2f6fe4", "#c9318d", "#f6466f", "#9b4fd6", "#ff8a3d"].map((h) => lin3(hexRgb(h)));
const LOOKS3 = ["chrome", "glass", "glossy", "lava", "iridescent"];
const LOOKS2 = ["layers", "gradient", "drops", "contour", "holo"];

function aces(c: number): number {
  const v = (c * (2.51 * c + 0.03)) / (c * (2.43 * c + 0.59) + 0.14);
  return enc(v < 0 ? 0 : v > 1 ? 1 : v);
}
function film(th: number): V3 {
  return [0.5 + 0.5 * cos(TAU * th), 0.5 + 0.5 * cos(TAU * (th + 0.33)), 0.5 + 0.5 * cos(TAU * (th + 0.67))];
}

// ---------------- 2D ----------------
interface Ball2 {
  x: number;
  y: number;
  r: number;
  c: number;
}
interface Group {
  b: Ball2[];
  bb: [number, number, number, number];
  c1: V3;
  c2: V3;
  ang: number;
  al: number;
}
function bbox(g: Group): void {
  let x0 = 1e9;
  let y0 = 1e9;
  let x1 = -1e9;
  let y1 = -1e9;
  for (const q of g.b) {
    x0 = Math.min(x0, q.x - q.r);
    y0 = Math.min(y0, q.y - q.r);
    x1 = Math.max(x1, q.x + q.r);
    y1 = Math.max(y1, q.y + q.r);
  }
  g.bb = [x0, y0, x1, y1];
}
function newGroup(): Group {
  return { b: [], bb: [0, 0, 0, 0], c1: [0, 0, 0], c2: [0, 0, 0], ang: 0, al: 1 };
}
function layout2D(R: Rand, N: number, size: number, asp: number): Ball2[] {
  const B: Ball2[] = [];
  const nc = Math.max(1, round(N / 3.2));
  const cs: [number, number][] = [];
  for (let c = 0; c < nc; c++) {
    let bx = 0;
    let by = 0;
    let bd = -1;
    for (let t = 0; t < 14; t++) {
      const x = asp * (0.12 + 0.76 * R());
      const y = 0.15 + 0.7 * R();
      let m = 9;
      for (const q of cs) m = Math.min(m, hypot2(x - q[0], y - q[1]));
      if (m > bd) {
        bd = m;
        bx = x;
        by = y;
      }
    }
    cs.push([bx, by]);
  }
  for (let i = 0; i < N; i++) {
    const c = i % nc;
    const r = size * (0.06 + 0.12 * pow(R(), 1.3)) * (i < nc ? 1.35 : 1);
    let x: number;
    let y: number;
    const mates = B.filter((b) => b.c === c);
    if (!mates.length) {
      x = cs[c][0];
      y = cs[c][1];
    } else {
      const p = mates[Math.floor(R() * mates.length)];
      const a = R() * TAU;
      const dd = (p.r / 2.05 + r) * (0.95 + 0.45 * R());
      x = p.x + cos(a) * dd;
      y = p.y + sin(a) * dd;
    }
    x = clamp(x, r * 0.6, asp - r * 0.6);
    y = clamp(y, r * 0.6, 1 - r * 0.6);
    B.push({ x, y, r: r * 2.05, c });
  }
  return B;
}
const FO2 = new Float64Array(3);
function fieldBalls(bs: Ball2[], u: number, v: number, o: Float64Array): void {
  let F = 0;
  let Fx = 0;
  let Fy = 0;
  for (const b of bs) {
    const dx = u - b.x;
    const dy = v - b.y;
    const R2 = b.r * b.r;
    const q = (dx * dx + dy * dy) / R2;
    if (q >= 1) continue;
    const oo = 1 - q;
    F += oo * oo * oo;
    const gd = (-6 * oo * oo) / R2;
    Fx += gd * dx;
    Fy += gd * dy;
  }
  o[0] = F;
  o[1] = Fx;
  o[2] = Fy;
}

function render2D(seed: string, W: number, H: number, p: ParamValues, look: string): Uint8ClampedArray {
  const d = newImage(W, H);
  const R = stream(seed, "metaballs-2d");
  const S = Math.floor(R() * 1e6);
  const asp = W / H;
  const T = 0.42 - 0.3 * num(p, "merge");
  const N = round(num(p, "balls"));
  const sz = num(p, "size") / 0.5;
  const ang = num(p, "light") * DEG;
  const Lx = sin(ang) * 0.7;
  const Ly = -0.7;
  const groups: Group[] = [];
  const addGroup = (cx: number, cy: number, n: number, r0: number, spread: number): Group => {
    const g = newGroup();
    for (let i = 0; i < n; i++) {
      const r = r0 * (0.55 + 0.7 * R());
      let x = cx;
      let y = cy;
      if (i > 0) {
        const q = g.b[Math.floor(R() * g.b.length)];
        const a = R() * TAU;
        const dd = (q.r / 2.05 + r) * (spread * (0.6 + 0.6 * R()));
        x = q.x + cos(a) * dd;
        y = q.y + sin(a) * dd;
      }
      g.b.push({ x, y, r: r * 2.05, c: 0 });
    }
    bbox(g);
    groups.push(g);
    return g;
  };
  if (look === "layers") {
    const nb = Math.max(2, round(N / 4));
    for (let k = 0; k < nb; k++) {
      const cx = asp * (0.12 + 0.76 * R());
      const cy = 0.12 + 0.76 * R();
      addGroup(cx, cy, 4 + Math.floor(R() * 5), 0.1 * sz, 1.05);
    }
    const ns = round(N * 1.6);
    for (let k = 0; k < ns; k++) {
      const cx = asp * R();
      const cy = R();
      addGroup(cx, cy, 1, 0.02 * sz + 0.045 * sz * pow(R(), 1.5), 1);
    }
  } else if (look === "gradient") {
    const g = newGroup();
    let x = -0.04;
    let y = 0.3 + 0.4 * R();
    let a = (R() - 0.5) * 1.4;
    const n = clamp(round(N * 0.45), 4, 9);
    const stp = (asp * 0.95) / n;
    for (let i = 0; i < n; i++) {
      const r = (0.08 + 0.09 * R()) * sz * (i === n - 1 ? 1.5 : 1);
      g.b.push({ x, y, r: r * 2.05, c: 0 });
      a += (R() - 0.5) * 2.2;
      if (y < 0.3) a = Math.abs(a) * 0.8 + 0.25;
      if (y > 0.7) a = -Math.abs(a) * 0.8 - 0.25;
      a = clamp(a, -0.85, 0.85);
      x += cos(a) * stp;
      y += sin(a) * stp * 0.9;
    }
    bbox(g);
    groups.push(g);
  } else if (look === "drops") {
    const g = newGroup();
    for (let i = 0; i < N; i++) {
      const r = (0.035 + 0.06 * pow(R(), 1.3)) * sz;
      let x = 0;
      let y = 0;
      let ok = false;
      for (let t = 0; t < 30 && !ok; t++) {
        x = asp * 0.5 + (R() - 0.5) * asp * 0.62;
        y = 0.5 + (R() - 0.5) * 0.68;
        ok = true;
        for (const q of g.b) {
          if (!(hypot2(q.x - x, q.y - y) > (q.r / 2.05 + r) * (R() < 0.2 ? 0.9 : 1.6))) {
            ok = false;
            break;
          }
        }
      }
      g.b.push({ x, y, r: r * 2.05, c: 0 });
    }
    bbox(g);
    groups.push(g);
  } else {
    const g = newGroup();
    g.b = layout2D(R, N, num(p, "size"), asp);
    bbox(g);
    groups.push(g);
  }
  const dark = R() < 0.55;
  const bgA = lin3(dark ? [24, 24, 28] : [118, 10, 72]);
  const bgB = lin3(dark ? [20, 20, 24] : [106, 16, 170]);
  for (const g of groups) {
    g.c1 = VIV[Math.floor(R() * VIV.length)];
    g.c2 = VIV[Math.floor(R() * VIV.length)];
    g.ang = R() * TAU;
    g.al = g.b.length > 1 ? 0.55 + 0.2 * R() : 0.7 + 0.25 * R();
  }
  const all: Ball2[] = [];
  for (const g of groups) for (const b of g.b) all.push(b);
  const gradA = lin3([66, 84, 240]);
  const gradB = lin3([196, 40, 220]);
  const gradC = lin3([246, 24, 118]);
  const paper = lin3([241, 238, 230]);
  const ink = lin3([28, 28, 34]);
  const fo = R() * 10;
  const LV = [0.12, 0.2, 0.3, 0.45, 0.65, 0.85, 1.2, 1.7, 2.4, 3.4].map((k) => T * k);
  const bgD0 = lin3([232, 246, 234]);
  const bgD1 = lin3([214, 224, 236]);
  const o = FO2;
  const o2 = new Float64Array(3);
  const gc = new Float64Array(3);
  for (let y = 0; y < H; y++) {
    const v = y / H;
    for (let x = 0; x < W; x++) {
      const u = x / H;
      let r: number;
      let g: number;
      let b: number;
      if (look === "layers") {
        r = lerp(bgA[0], bgB[0], v);
        g = lerp(bgA[1], bgB[1], v);
        b = lerp(bgA[2], bgB[2], v);
        for (const G of groups) {
          if (u < G.bb[0] || u > G.bb[2] || v < G.bb[1] || v > G.bb[3]) continue;
          fieldBalls(G.b, u, v, o);
          if (o[0] < T * 0.5) continue;
          const gl = hypot2(o[1], o[2]) + 1e-9;
          const cov = clamp(((o[0] - T) / gl) * H + 0.5, 0, 1);
          if (cov <= 0) continue;
          const bx = G.bb;
          const t = clamp(((u - (bx[0] + bx[2]) / 2) * cos(G.ang) + (v - (bx[1] + bx[3]) / 2) * sin(G.ang)) / Math.max(bx[2] - bx[0], bx[3] - bx[1]) + 0.5, 0, 1);
          const a = cov * G.al;
          r = lerp(r, lerp(G.c1[0], G.c2[0], t), a);
          g = lerp(g, lerp(G.c1[1], G.c2[1], t), a);
          b = lerp(b, lerp(G.c1[2], G.c2[2], t), a);
        }
      } else if (look === "gradient") {
        r = 0.79;
        g = 0.79;
        b = 0.8;
        const G = groups[0];
        if (!(u < G.bb[0] || u > G.bb[2] || v < G.bb[1] || v > G.bb[3])) {
          fieldBalls(G.b, u, v, o);
          const gl = hypot2(o[1], o[2]) + 1e-9;
          const cov = clamp(((o[0] - T) / gl) * H + 0.5, 0, 1);
          if (cov > 0) {
            const t = clamp((v - G.bb[1]) / (G.bb[3] - G.bb[1]), 0, 1);
            for (let c = 0; c < 3; c++) gc[c] = t < 0.5 ? lerp(gradA[c], gradB[c], t * 2) : lerp(gradB[c], gradC[c], t * 2 - 1);
            r = lerp(r, gc[0], cov);
            g = lerp(g, gc[1], cov);
            b = lerp(b, gc[2], cov);
          }
        }
      } else if (look === "drops") {
        const t = clamp((u / asp) * 0.6 + v * 0.4, 0, 1);
        r = lerp(bgD0[0], bgD1[0], t);
        g = lerp(bgD0[1], bgD1[1], t);
        b = lerp(bgD0[2], bgD1[2], t);
        fieldBalls(all, u - Lx * 0.012 * sz, v - Ly * 0.012 * sz, o2);
        const sh = sstep(T * 0.3, T * 1.3, o2[0]);
        fieldBalls(all, u, v, o);
        const F = o[0];
        const gl = hypot2(o[1], o[2]) + 1e-9;
        const cov = clamp(((F - T) / gl) * H + 0.5, 0, 1);
        const k = 1 - 0.07 * sh * (1 - cov);
        r *= k;
        g *= k;
        b *= k;
        if (cov > 0) {
          const sq = 2 * Math.sqrt(Math.max(F - T, 1e-6));
          const nx = (-o[1] * 0.09 * sz) / sq;
          const ny = (-o[2] * 0.09 * sz) / sq;
          const nl = hypot3(nx, ny, 1);
          const ex = 1 - 1 / nl;
          const base = [r, g, b];
          const rr = [0, 0, 0];
          for (let c = 0; c < 3; c++) {
            const o_ = (c - 1) * 0.05;
            const dk = 0.62 * sstep(0.45 + o_, 0.9 + o_, ex) * (1 - 0.45 * sstep(0.9 + o_, 1, ex));
            rr[c] = base[c] * (1 - dk);
          }
          const lit = (nx * Lx + ny * Ly) / nl;
          const q1 = (ex - 0.12) / 0.05;
          const q2 = (ex - 0.35) / 0.25;
          const hl = pow(clamp(lit * 1.6, 0, 1), 6) * sstep(0.3, 0.7, ex) * 0.55 + exp(-q1 * q1) * clamp(-lit, 0, 1) * 0.12;
          const ca = exp(-q2 * q2) * clamp(-lit * 2, 0, 1) * 0.08;
          r = lerp(r, rr[0] + hl + ca, cov);
          g = lerp(g, rr[1] + hl + ca, cov);
          b = lerp(b, rr[2] + hl + ca, cov);
        }
      } else if (look === "contour") {
        fieldBalls(all, u, v, o);
        const F = o[0];
        const gl = hypot2(o[1], o[2]) + 1e-9;
        const cov = clamp(((F - T) / gl) * H + 0.5, 0, 1);
        r = paper[0] - cov * 0.06;
        g = paper[1] - cov * 0.06;
        b = paper[2] - cov * 0.048;
        for (const L of LV) {
          const dpx = (Math.abs(F - L) / gl) * H;
          const main = L === LV[3] ? 0 : 1;
          const wd = L === T ? 1.3 : 0.55;
          const a = clamp(wd - dpx + 0.5, 0, 1) * (L === T ? 1 : 0.75) * (main + 1 - main);
          r = lerp(r, ink[0], a);
          g = lerp(g, ink[1], a);
          b = lerp(b, ink[2], a);
        }
      } else {
        fieldBalls(all, u, v, o);
        const F = o[0];
        const gl = hypot2(o[1], o[2]) + 1e-9;
        const cov = clamp(((F - T) / gl) * H + 0.5, 0, 1);
        const fc = film(fo + (F / T) * 0.55);
        const gw = clamp(F / T, 0, 1) * 0.35 * (1 - cov);
        const kk = 0.75 + 0.25 * sin((F / T) * 9);
        r = lerp(0.012 + fc[0] * gw * 0.4, fc[0] * kk, cov);
        g = lerp(0.012 + fc[1] * gw * 0.4, fc[1] * kk, cov);
        b = lerp(0.02 + fc[2] * gw * 0.4, fc[2] * kk, cov);
      }
      const gr = look === "gradient" ? 0 : (hash2(x, y, S) - 0.5) * 0.012;
      put(d, y * W + x, enc(r + gr), enc(g + gr), enc(b + gr));
    }
  }
  return d;
}

// ---------------- 3D ----------------
function box(az: number, el: number, a0: number, a1: number, e0: number, e1: number, k: number): number {
  return sstep(a0 - 0.06, a0, az) * sstep(a1 + 0.06, a1, az) * sstep(e0 - 0.04, e0, el) * sstep(e1 + 0.04, e1, el) * k;
}
function env3(dx: number, dy: number, dz: number, warm: boolean): V3 {
  const el = dy;
  const az = atan2(dx, dz);
  let c: number;
  if (el > 0) {
    c = lerp(0.05, 0.22, pow(el, 0.7));
    c += box(az, el, -1.3, -0.5, 0.2, 0.7, 6) + box(az, el, 0.5, 1.1, 0.12, 0.5, 3.4) + box(az, el, -0.4, 0.4, 0.82, 1.2, 5) + box(az, el, 2.2, 2.9, 0.05, 0.45, 2.6);
  } else c = lerp(0.1, 0.03, Math.min(1, -el * 3));
  return warm ? [c * 1.04, c, c * 0.94] : [c * 0.96, c, c * 1.05];
}
interface Ball3 {
  x: number;
  y: number;
  z: number;
  rr: number;
  R: number;
  R2: number;
  c: number;
}

class Scene3 {
  B: Ball3[] = [];
  Gs: number[] = [];
  T = 0;
  Lx = 0;
  Ly = 0;
  Lz = 0;
  glass = false;
  LST: Int32Array = new Int32Array(0);
  TN = 0;
  FO = new Float64Array(7);
  field(px: number, py: number, pz: number, n: number): void {
    let F = 0;
    let gx = 0;
    let gy = 0;
    let gz = 0;
    let w0 = 0;
    let w1 = 0;
    let w2 = 0;
    for (let k = 0; k < n; k++) {
      const b = this.B[this.LST[k]];
      const dx = px - b.x;
      const dy = py - b.y;
      const dz = pz - b.z;
      const q = (dx * dx + dy * dy + dz * dz) / b.R2;
      if (q >= 1) continue;
      const o = 1 - q;
      const g = o * o * o;
      const gd = (-6 * o * o) / b.R2;
      F += g;
      gx += gd * dx;
      gy += gd * dy;
      gz += gd * dz;
      const c = b.c % 3;
      if (c === 0) w0 += g;
      else if (c === 1) w1 += g;
      else w2 += g;
    }
    const O = this.FO;
    O[0] = F;
    O[1] = gx;
    O[2] = gy;
    O[3] = gz;
    O[4] = w0;
    O[5] = w1;
    O[6] = w2;
  }
  trace(ox: number, oy: number, oz: number, dx: number, dy: number, dz: number, tMax: number, skip: number): number {
    let n = 0;
    let t0 = 1e9;
    let t1 = -1;
    const B = this.B;
    for (let i = 0; i < B.length; i++) {
      const b = B[i];
      const cx = b.x - ox;
      const cy = b.y - oy;
      const cz = b.z - oz;
      const tc = cx * dx + cy * dy + cz * dz;
      const d2 = cx * cx + cy * cy + cz * cz - tc * tc;
      if (d2 >= b.R2) continue;
      const h = Math.sqrt(b.R2 - d2);
      const a = tc - h;
      const e = tc + h;
      if (e <= 0.001) continue;
      this.LST[n++] = i;
      if (a < t0) t0 = a;
      if (e > t1) t1 = e;
    }
    if (n === 0) return -1;
    let t = Math.max(t0, skip > 0 ? skip : 0.001);
    let G = 0;
    let maxStep = 1e9;
    for (let k = 0; k < n; k++) {
      G += this.Gs[this.LST[k]];
      maxStep = Math.min(maxStep, B[this.LST[k]].rr);
    }
    maxStep *= 0.35;
    t1 = Math.min(t1, tMax);
    let prev = t;
    for (let s = 0; s < 600 && t < t1; s++) {
      this.field(ox + dx * t, oy + dy * t, oz + dz * t, n);
      if (this.FO[0] >= this.T) {
        let a = prev;
        let b = t;
        for (let k = 0; k < 7; k++) {
          const m = (a + b) / 2;
          this.field(ox + dx * m, oy + dy * m, oz + dz * m, n);
          if (this.FO[0] >= this.T) b = m;
          else a = m;
        }
        this.TN = n;
        return b;
      }
      prev = t;
      t += clamp(((this.T - this.FO[0]) / G) * 0.9, 0.004, maxStep);
    }
    return -1;
  }
  /** Soft shadow [0] and contact occlusion [1] from the blobs (sphere approximation). */
  shadow(px: number, py: number, pz: number, self: boolean, out: Float64Array): void {
    let s = 1;
    let ao = 1;
    for (const b of this.B) {
      const re = b.rr * 1.15;
      const cx = b.x - px;
      const cy = b.y - py;
      const cz = b.z - pz;
      const dist = hypot3(cx, cy, cz);
      if (self && dist < re * 1.25) continue;
      const tc = cx * this.Lx + cy * this.Ly + cz * this.Lz;
      if (tc > 0) {
        const dp = Math.sqrt(Math.max(0, dist * dist - tc * tc));
        const k = clamp((dp - re) / (tc * 0.32 + 0.02) + 0.5, 0, 1);
        s *= this.glass ? 1 - 0.45 * (1 - k) : k;
      }
      const q = re / Math.max(dist, 1e-3);
      ao *= 1 - 0.75 * Math.min(1, q * q) * clamp((cy / dist) * 0.5 + 0.6, 0, 1);
    }
    out[0] = s;
    out[1] = ao;
  }
}

function render3D(seed: string, W: number, H: number, p: ParamValues, look: string): Uint8ClampedArray {
  const R = stream(seed, "metaballs-3d");
  const S = Math.floor(R() * 1e6);
  const lava = look === "lava";
  const glass = look === "glass";
  const N = round(num(p, "balls"));
  const size = num(p, "size");
  const sc = new Scene3();
  sc.glass = glass;
  const B = sc.B;
  const nc = Math.max(1, round(N / 3.2));
  const cs: [number, number][] = [];
  for (let c = 0; c < nc; c++) {
    let bx = 0;
    let bz = 0;
    let bd = -1;
    for (let t = 0; t < 12; t++) {
      const x = -1.9 + 3.8 * R();
      const z = -1.6 + 2.6 * R();
      let m = 9;
      for (const q of cs) m = Math.min(m, hypot2(x - q[0], z - q[1]));
      if (m > bd) {
        bd = m;
        bx = x;
        bz = z;
      }
    }
    cs.push([bx, bz]);
  }
  for (let i = 0; i < N; i++) {
    const c = i % nc;
    const r = size * (0.32 + 0.5 * pow(R(), 1.3)) * (i < nc ? 1.3 : 1);
    const mates = B.filter((b) => b.c === c);
    let x: number;
    let y: number;
    let z: number;
    if (!mates.length) {
      x = cs[c][0];
      z = cs[c][1];
      y = lava ? 0.4 + 2.6 * R() : r * 0.9;
    } else {
      const q = mates[Math.floor(R() * mates.length)];
      const a = R() * TAU;
      const e = (R() - 0.3) * 1.2;
      const dd = (q.rr + r) * (0.85 + 0.3 * R());
      x = q.x + cos(a) * cos(e) * dd;
      z = q.z + sin(a) * cos(e) * dd * 0.7;
      y = Math.max(lava ? 0.2 : r * 0.85, q.y + sin(e) * dd);
    }
    B.push({ x, y, z, rr: r, R: r * 2.05, R2: r * r * 4.2025, c });
  }
  sc.Gs = B.map((b) => 1.9 / b.R);
  sc.LST = new Int32Array(Math.max(1, N));
  sc.T = 0.42 - 0.3 * num(p, "merge");
  const T = sc.T;
  const ang = num(p, "light") * DEG;
  {
    let Lx = sin(ang) * 0.7;
    let Ly = 0.65;
    let Lz = cos(ang) * 0.45 + 0.2;
    const l = hypot3(Lx, Ly, Lz);
    Lx /= l;
    Ly /= l;
    Lz /= l;
    sc.Lx = Lx;
    sc.Ly = Ly;
    sc.Lz = Lz;
  }
  const Lx = sc.Lx;
  const Ly = sc.Ly;
  const Lz = sc.Lz;
  const lv = LAVA[Math.floor(R() * LAVA.length)];
  const gp = GLOSS[Math.floor(R() * GLOSS.length)].map(lin3);
  const bg0 = lin3(lv.bg[0]);
  const bg1 = lin3(lv.bg[1]);
  const wax = lin3(lv.wax);
  const fo = R() * 10;
  const ex0 = 0;
  const ey0 = lava ? 1.7 : 1.9;
  const ez0 = lava ? 7.2 : 6;
  const fa = ((lava ? 34 : 30) / 2) * DEG;
  const fov = sin(fa) / cos(fa);
  let fx = -ex0;
  let fy = (lava ? 1.5 : 0.6) - ey0;
  let fz = -ez0;
  {
    const l = hypot3(fx, fy, fz);
    fx /= l;
    fy /= l;
    fz /= l;
  }
  let rx = -fz;
  let rz = fx;
  {
    const l = hypot2(rx, rz);
    rx /= l;
    rz /= l;
  }
  const ux = -rz * fy;
  const uy = rz * fx - rx * fz;
  const uz = rx * fy;
  const asp = W / H;
  const warm = look !== "iridescent";
  const floorC: V3 = look === "glossy" || glass ? lin3(glass ? [214, 224, 232] : [236, 230, 222]) : look === "chrome" ? [0.05, 0.05, 0.055] : [0.03, 0.03, 0.045];
  const SH = new Float64Array(2);
  const wall = (dy: number): V3 => {
    const k = 1 - 0.5 * sstep(0, 0.45, dy);
    return look === "glossy" || glass ? [floorC[0] * 0.95 * k, floorC[1] * 0.95 * k, floorC[2] * 0.95 * k] : [floorC[0] * 1.8 * k, floorC[1] * 1.8 * k, floorC[2] * 2 * k];
  };
  const floorAt = (qx: number, qz: number, dist: number): V3 => {
    sc.shadow(qx, 0, qz, false, SH);
    const lit = (0.22 + 0.78 * SH[0] * Ly) * SH[1];
    const fade = sstep(5, 22, dist);
    const bgc = wall(0);
    return [lerp(floorC[0] * lit * 2.2, bgc[0], fade), lerp(floorC[1] * lit * 2.2, bgc[1], fade), lerp(floorC[2] * lit * 2.2, bgc[2], fade)];
  };
  const normal = (): V3 => {
    const O = sc.FO;
    const l = hypot3(O[1], O[2], O[3]) || 1;
    return [-O[1] / l, -O[2] / l, -O[3] / l];
  };
  const reflectCol = (px: number, py: number, pz: number, dx: number, dy: number, dz: number, depth: number): V3 => {
    const th = depth < 1 ? sc.trace(px, py, pz, dx, dy, dz, 30, 0.03) : -1;
    const tf = dy < 0 ? -py / dy : 1e9;
    if (th > 0 && th < tf) {
      sc.field(px + dx * th, py + dy * th, pz + dz * th, sc.TN);
      const [nx, ny, nz] = normal();
      const dn = dx * nx + dy * ny + dz * nz;
      const e = env3(dx - 2 * dn * nx, dy - 2 * dn * ny, dz - 2 * dn * nz, warm);
      return [e[0] * 0.7, e[1] * 0.7, e[2] * 0.7];
    }
    if (tf < 1e8) return floorAt(px + dx * tf, pz + dz * tf, hypot3(dx * tf, dy * tf, dz * tf));
    return env3(dx, dy, dz, warm);
  };
  const tint = (hx: number, hy: number, hz: number, extra: number): V3 => {
    const f = film(fo + hy * 0.35 + fbm2(hx * 0.6, hz * 0.6, S + 9, 3) * 1.4 + extra);
    return [0.15 + f[0] * 1.05, 0.15 + f[1] * 1.05, 0.15 + f[2] * 1.05];
  };
  const OUT = new Float64Array(4);
  const shade = (sx: number, sy: number): void => {
    const px = (sx / W * 2 - 1) * fov * asp;
    const py = (1 - sy / H * 2) * fov;
    let dx = fx + rx * px + ux * py;
    let dy = fy + uy * py;
    let dz = fz + rz * px + uz * py;
    {
      const l = hypot3(dx, dy, dz);
      dx /= l;
      dy /= l;
      dz /= l;
    }
    const th = sc.trace(ex0, ey0, ez0, dx, dy, dz, 60, 0);
    if (th < 0) {
      OUT[3] = 0;
      if (lava) {
        const v = sy / H;
        const u = sx / W;
        const k = 0.75 + 0.5 * fbm2(u * 2 + 3, v * 1.2, S + 3, 4);
        const gq = (v - 1.05) * 2.2;
        const glow = exp(-gq * gq) * 0.9;
        const vg = 1 - 0.55 * pow(Math.abs(u - 0.5) * 2, 2.4);
        OUT[0] = (lerp(bg1[0], bg0[0], v) * k + wax[0] * glow * 0.5) * vg;
        OUT[1] = (lerp(bg1[1], bg0[1], v) * k + wax[1] * glow * 0.35) * vg;
        OUT[2] = (lerp(bg1[2], bg0[2], v) * k + wax[2] * glow * 0.2) * vg;
        return;
      }
      if (dy < 0) {
        const tf = -ey0 / dy;
        const qx = ex0 + dx * tf;
        const qz = ez0 + dz * tf;
        const c = floorAt(qx, qz, tf);
        if (look !== "glossy" && !glass) {
          const fr = 0.06 + 0.5 * pow(1 + dy, 5);
          const rc = sc.trace(qx, 0.001, qz, dx, -dy, dz, 30, 0);
          if (rc > 0) {
            const hx = qx + dx * rc;
            const hy = -dy * rc;
            const hz = qz + dz * rc;
            sc.field(hx, hy, hz, sc.TN);
            const [nx, ny, nz] = normal();
            const dn = dx * nx - dy * ny + dz * nz;
            const e = env3(dx - 2 * dn * nx, -dy - 2 * dn * ny, dz - 2 * dn * nz, warm);
            const k: V3 = look === "iridescent" ? tint(hx, hy, hz, 0.4) : [1, 1, 1];
            const fade = exp(-hy * 1.1) * (fr * 0.6 + 0.12);
            c[0] += e[0] * k[0] * fade;
            c[1] += e[1] * k[1] * fade;
            c[2] += e[2] * k[2] * fade;
          }
        }
        OUT[0] = c[0];
        OUT[1] = c[1];
        OUT[2] = c[2];
        return;
      }
      const w = wall(dy);
      OUT[0] = w[0];
      OUT[1] = w[1];
      OUT[2] = w[2];
      return;
    }
    OUT[3] = 1;
    const hx = ex0 + dx * th;
    const hy = ey0 + dy * th;
    const hz = ez0 + dz * th;
    sc.field(hx, hy, hz, sc.TN);
    const w0 = sc.FO[4];
    const w1 = sc.FO[5];
    const w2 = sc.FO[6];
    const [nx, ny, nz] = normal();
    const ndv = Math.max(0, -(dx * nx + dy * ny + dz * nz));
    const fr = 0.04 + 0.96 * pow(1 - ndv, 5);
    const dn = dx * nx + dy * ny + dz * nz;
    const rdx = dx - 2 * dn * nx;
    const rdy = dy - 2 * dn * ny;
    const rdz = dz - 2 * dn * nz;
    const ndl = nx * Lx + ny * Ly + nz * Lz;
    const hvx = Lx - dx;
    const hvy = Ly - dy;
    const hvz = Lz - dz;
    const ndh = Math.max(0, (nx * hvx + ny * hvy + nz * hvz) / hypot3(hvx, hvy, hvz));
    const tn = sc.TN;
    sc.field(hx + nx * 0.12, hy + ny * 0.12, hz + nz * 0.12, tn);
    const ao1 = clamp(1 - (sc.FO[0] - T * 0.5) * 1.6, 0.25, 1);
    if (look === "chrome" || look === "iridescent") {
      const k = 0.78 + 0.22 * fr;
      const tt: V3 = look === "iridescent" ? tint(hx, hy, hz, (1 - ndv) * 0.9) : [1, 1, 1];
      sc.shadow(hx, hy, hz, true, SH);
      const sp = pow(ndh, 500) * 30 * SH[0];
      const e = reflectCol(hx + nx * 0.005, hy + ny * 0.005, hz + nz * 0.005, rdx, rdy, rdz, 0);
      OUT[0] = e[0] * k * tt[0] * ao1 + sp;
      OUT[1] = e[1] * k * tt[1] * ao1 + sp;
      OUT[2] = e[2] * k * tt[2] * ao1 + sp;
    } else if (glass) {
      const eta = 1 / 1.45;
      const cI = -dn;
      const k1 = Math.sqrt(Math.max(0, 1 - eta * eta * (1 - cI * cI)));
      const tx = eta * dx + (eta * cI - k1) * nx;
      const ty = eta * dy + (eta * cI - k1) * ny;
      const tz = eta * dz + (eta * cI - k1) * nz;
      let t2 = 0.02;
      let qx = hx;
      let qy = hy;
      let qz = hz;
      for (let k = 0; k < 80; k++) {
        t2 += 0.025;
        qx = hx + tx * t2;
        qy = hy + ty * t2;
        qz = hz + tz * t2;
        sc.field(qx, qy, qz, tn);
        if (sc.FO[0] < T) break;
      }
      sc.field(qx, qy, qz, tn);
      const ml = hypot3(sc.FO[1], sc.FO[2], sc.FO[3]) || 1;
      const mx = sc.FO[1] / ml;
      const my = sc.FO[2] / ml;
      const mz = sc.FO[3] / ml;
      const c2 = -(tx * mx + ty * my + tz * mz);
      const eta2 = 1.45;
      const k2 = 1 - eta2 * eta2 * (1 - c2 * c2);
      let ox: number;
      let oy: number;
      let oz: number;
      if (k2 < 0) {
        const dd = tx * mx + ty * my + tz * mz;
        ox = tx - 2 * dd * mx;
        oy = ty - 2 * dd * my;
        oz = tz - 2 * dd * mz;
      } else {
        const s2 = Math.sqrt(k2);
        ox = eta2 * tx - (eta2 * c2 - s2) * mx;
        oy = eta2 * ty - (eta2 * c2 - s2) * my;
        oz = eta2 * tz - (eta2 * c2 - s2) * mz;
      }
      const e = env3(rdx, rdy, rdz, true);
      const fk = 0.04 + 0.96 * pow(1 - ndv, 5);
      const sp = pow(ndh, 400) * 25;
      for (let c = 0; c < 3; c++) {
        const s = 1 + (c - 1) * 0.06;
        const ax = dx + (ox - dx) * s;
        const ay = dy + (oy - dy) * s;
        const az = dz + (oz - dz) * s;
        const al = hypot3(ax, ay, az);
        const bc = reflectCol(qx, qy, qz, ax / al, ay / al, az / al, 1);
        OUT[c] = bc[c] * 0.93 * (1 - fk) + e[c] * fk * 1.4 + sp;
      }
    } else if (look === "glossy") {
      const ws = w0 + w1 + w2 + 1e-9;
      sc.shadow(hx, hy, hz, true, SH);
      const s = SH[0];
      const dif = Math.max(0, ndl) * s * 1.25;
      const amb = (0.32 + 0.22 * ny) * ao1 * SH[1];
      const wrap = (Math.max(0, ndl + 0.35) / 1.35) * 0.18;
      const e = reflectCol(hx + nx * 0.005, hy + ny * 0.005, hz + nz * 0.005, rdx, rdy, rdz, 1);
      const cc = 0.04 + 0.96 * pow(1 - ndv, 5);
      const sp = pow(ndh, 180) * 9 * s + pow(ndh, 28) * 0.3 * s;
      for (let c = 0; c < 3; c++) {
        const al = (gp[0][c] * w0 + gp[1][c] * w1 + gp[2][c] * w2) / ws;
        OUT[c] = al * (dif + amb + wrap) + e[c] * cc * 0.9 + sp;
      }
    } else {
      let t2 = th + 0.01;
      let thick = 0;
      for (let k = 0; k < 40; k++) {
        t2 += 0.04;
        sc.field(ex0 + dx * t2, ey0 + dy * t2, ez0 + dz * t2, tn);
        if (sc.FO[0] < T) break;
        thick += 0.04;
      }
      const thin = exp(-thick * 2.2);
      const below = clamp(0.45 - 0.7 * ny, 0, 1.2);
      const core = 1 - thin;
      const sat = 0.55 + 0.45 * core;
      const glow = (0.45 + 1.1 * below) * (0.7 + 0.9 * thin) + 0.25 * Math.max(0, ndl);
      const rim = pow(1 - ndv, 3) * 0.6;
      const sp = pow(ndh, 70) * 1.2 + fr * 0.25;
      OUT[0] = wax[0] * glow + wax[0] * rim + sp;
      OUT[1] = wax[1] * glow * (1 - 0.35 * core * sat) + wax[1] * rim * 0.8 + sp;
      OUT[2] = wax[2] * glow * (1 - 0.6 * core * sat) + wax[2] * rim * 0.6 + sp;
    }
  };
  const hit = new Uint8Array(W * H);
  const buf = new Float64Array(W * H * 3);
  for (let y = 0; y < H; y++)
    for (let x = 0; x < W; x++) {
      shade(x + 0.5, y + 0.5);
      const i = y * W + x;
      buf[i * 3] = OUT[0];
      buf[i * 3 + 1] = OUT[1];
      buf[i * 3 + 2] = OUT[2];
      hit[i] = OUT[3];
    }
  const offs = [0.25, 0.25, 0.75, 0.25, 0.25, 0.75, 0.75, 0.75];
  for (let y = 1; y < H - 1; y++)
    for (let x = 1; x < W - 1; x++) {
      const i = y * W + x;
      const h = hit[i];
      if (hit[i - 1] === h && hit[i + 1] === h && hit[i - W] === h && hit[i + W] === h) continue;
      let r = 0;
      let g = 0;
      let b = 0;
      for (let k = 0; k < 8; k += 2) {
        shade(x + offs[k], y + offs[k + 1]);
        r += OUT[0];
        g += OUT[1];
        b += OUT[2];
      }
      buf[i * 3] = r / 4;
      buf[i * 3 + 1] = g / 4;
      buf[i * 3 + 2] = b / 4;
    }
  const ex = lava ? 1.05 : look === "glossy" ? 1.15 : glass ? 1.05 : 1.5;
  const d = newImage(W, H);
  for (let y = 0; y < H; y++)
    for (let x = 0; x < W; x++) {
      const i = y * W + x;
      const gr = (hash2(x, y, S) - 0.5) * 0.004;
      let r = aces(buf[i * 3] * ex + gr);
      let g = aces(buf[i * 3 + 1] * ex + gr);
      let b = aces(buf[i * 3 + 2] * ex + gr);
      if (lava) {
        // glass of the lamp: soft vertical highlights
        const u = (x + 0.5) / W;
        let a = 0;
        const GS = [0.15, 0.03, 0.05, 0.2, 0.008, 0.045, 0.88, 0.06, 0.02];
        for (let k = 0; k < 9; k += 3) {
          const t = Math.abs(u - GS[k]) / (GS[k + 1] * 2);
          if (t < 1) a += GS[k + 2] * (1 - t);
        }
        r += 255 * a;
        g += 255 * a;
        b += 255 * a;
      }
      put(d, i, r, g, b);
    }
  return d;
}

export const metaballs: Generator = {
  id: "metaballs",
  title: "Metaballs",
  description: "Blobs that melt together: ray-marched 3D chrome, glass, plastic, lava and oil film, or 2D graphic looks.",
  aspect: 1.5,
  params: [
    {
      kind: "choice",
      key: "look",
      label: "Look",
      default: "chrome",
      options: [
        opt("chrome", "Chrome", "3D"),
        opt("glass", "Glass", "3D"),
        opt("glossy", "Glossy", "3D"),
        opt("lava", "Lava", "3D"),
        opt("iridescent", "Iridescent", "3D"),
        opt("layers", "Layers", "2D"),
        opt("gradient", "Gradient", "2D"),
        opt("drops", "Drops", "2D"),
        opt("contour", "Contour", "2D"),
        opt("holo", "Holo", "2D"),
      ],
    },
    { kind: "number", key: "balls", label: "Blobs", min: 3, max: 22, step: 1, default: 10 },
    { kind: "number", key: "size", label: "Size", min: 0.25, max: 0.8, step: 0.01, default: 0.5, unit: "%" },
    { kind: "number", key: "merge", label: "Merge", min: 0, max: 1, step: 0.01, default: 0.55, unit: "%" },
    { kind: "number", key: "light", label: "Light", min: -80, max: 80, step: 1, default: -30, unit: "°" },
  ],
  previews: [
    { title: "3D looks", sub: "Ray-marched surfaces on a studio floor.", key: "look", values: LOOKS3, labels: ["Chrome", "Glass", "Glossy", "Lava", "Iridescent"] },
    { title: "2D looks", sub: "The same idea as flat graphics.", key: "look", values: LOOKS2, labels: ["Layers", "Gradient", "Drops", "Contour", "Holo"] },
  ],
  render(seed, W, H, p) {
    const look = str(p, "look");
    return LOOKS2.includes(look) ? render2D(seed, W, H, p, look) : render3D(seed, W, H, p, look);
  },
};
