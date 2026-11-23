/** Sky: Rayleigh/Mie/ozone single scattering, volumetric clouds, cirrus, sun, moon and stars. */
import { DEG, PI, acos, asin, atan2, clamp, cos, exp, hash2, newImage, pow, put, round, sin, sstep, stream, type Rand, type V3 } from "./base.js";
import { type Generator, num, opt, str } from "./spec.js";

// ---------- gradient noise (per seed permutation) ----------
const GX = [1, -1, 1, -1, 1, -1, 0, 0];
const GY = [1, 1, -1, -1, 0, 0, 1, -1];
class GNoise {
  p = new Uint8Array(512);
  constructor(R: Rand) {
    const a = new Uint8Array(256);
    for (let i = 0; i < 256; i++) a[i] = i;
    for (let i = 255; i > 0; i--) {
      const j = Math.floor(R() * (i + 1));
      const t = a[i];
      a[i] = a[j];
      a[j] = t;
    }
    for (let i = 0; i < 512; i++) this.p[i] = a[i & 255];
  }
  n2(x: number, y: number): number {
    const p = this.p;
    const xi = Math.floor(x);
    const yi = Math.floor(y);
    const xf = x - xi;
    const yf = y - yi;
    const X = xi & 255;
    const Y = yi & 255;
    const g00 = p[p[X] + Y] & 7;
    const g01 = p[p[X] + Y + 1] & 7;
    const g10 = p[p[X + 1] + Y] & 7;
    const g11 = p[p[X + 1] + Y + 1] & 7;
    const u = xf * xf * xf * (xf * (xf * 6 - 15) + 10);
    const v = yf * yf * yf * (yf * (yf * 6 - 15) + 10);
    const d00 = GX[g00] * xf + GY[g00] * yf;
    const d10 = GX[g10] * (xf - 1) + GY[g10] * yf;
    const d01 = GX[g01] * xf + GY[g01] * (yf - 1);
    const d11 = GX[g11] * (xf - 1) + GY[g11] * (yf - 1);
    const a0 = d00 + u * (d10 - d00);
    const b0 = d01 + u * (d11 - d01);
    return a0 + v * (b0 - a0);
  }
  fbm(x: number, y: number, oct: number): number {
    let s = 0;
    let amp = 0.5;
    let tot = 0;
    for (let o = 0; o < oct; o++) {
      s += amp * (this.n2(x, y) * 0.78 + 0.5);
      tot += amp;
      amp *= 0.5;
      const nx = (0.8 * x + 0.6 * y) * 2.03 + 17.3;
      y = (-0.6 * x + 0.8 * y) * 2.03 + 9.1;
      x = nx;
    }
    return s / tot;
  }
}

// ---------- physical atmosphere (km) ----------
const RE = 6360;
const RA = 6460;
const ESUN = 20;
const BR = [5.802e-3, 13.558e-3, 33.1e-3];
const BO = [0.65e-3, 1.881e-3, 0.085e-3];
function sphT(px: number, py: number, pz: number, dx: number, dy: number, dz: number, r: number): number {
  const b = px * dx + py * dy + pz * dz;
  const c = px * px + py * py + pz * pz - r * r;
  const d = b * b - c;
  if (d < 0) return -1;
  const s = Math.sqrt(d);
  const t1 = -b - s;
  const t2 = -b + s;
  return t1 > 0 ? t1 : t2 > 0 ? t2 : -1;
}
function hitsGround(px: number, py: number, pz: number, dx: number, dy: number, dz: number): boolean {
  const b = px * dx + py * dy + pz * dz;
  if (b > 0) return false;
  const c = px * px + py * py + pz * pz - RE * RE;
  return b * b - c > 0;
}
function odTo(px: number, py: number, pz: number, dx: number, dy: number, dz: number, n: number): V3 {
  const tm = sphT(px, py, pz, dx, dy, dz, RA);
  let r = 0;
  let m = 0;
  let o = 0;
  if (tm <= 0) return [0, 0, 0];
  for (let i = 0; i < n; i++) {
    const a = i / n;
    const b = (i + 1) / n;
    const t = (tm * (a + b) * (a + b)) / 4;
    const dt = tm * (b * b - a * a);
    const x = px + dx * t;
    const y = py + dy * t;
    const z = pz + dz * t;
    const h = Math.sqrt(x * x + y * y + z * z) - RE;
    r += exp(-h / 8) * dt;
    m += exp(-h / 1.2) * dt;
    o += Math.max(0, 1 - Math.abs(h - 25) / 15) * dt;
  }
  return [r, m, o];
}
function extinct(od: V3, mK: number): V3 {
  const me = 4.4e-3 * mK;
  return [exp(-(BR[0] * od[0] + me * od[1] + BO[0] * od[2])), exp(-(BR[1] * od[0] + me * od[1] + BO[1] * od[2])), exp(-(BR[2] * od[0] + me * od[1] + BO[2] * od[2]))];
}
const NE = 72;
const NA = 40;
function buildLUT(lightEl: number, mK: number): Float64Array {
  const L = new Float64Array(NE * NA * 3);
  const sz = cos(lightEl * DEG);
  const sy = sin(lightEl * DEG);
  const oy = RE + 0.05;
  const g = 0.76;
  const ms = 3.996e-3 * mK;
  const me = 4.4e-3 * mK;
  for (let ie = 0; ie < NE; ie++) {
    const u = ie / (NE - 1);
    const el = Math.max(0.4, 90 * u * u);
    const cv = cos(el * DEG);
    const vy = sin(el * DEG);
    for (let ia = 0; ia < NA; ia++) {
      const az = (ia / (NA - 1)) * PI;
      const vx = cv * sin(az);
      const vz = cv * cos(az);
      const mu = vy * sy + vz * sz;
      const pR = (3 / (16 * PI)) * (1 + mu * mu);
      const pM = ((3 / (8 * PI)) * ((1 - g * g) * (1 + mu * mu))) / ((2 + g * g) * pow(1 + g * g - 2 * g * mu, 1.5));
      const tm = sphT(0, oy, 0, vx, vy, vz, RA);
      const n = 22;
      const acc = [0, 0, 0];
      let odR = 0;
      let odM = 0;
      let odO = 0;
      for (let i = 0; i < n; i++) {
        const a = i / n;
        const c = (i + 1) / n;
        const t = (tm * (a + c) * (a + c)) / 4;
        const dt = tm * (c * c - a * a);
        const x = vx * t;
        const y = oy + vy * t;
        const z = vz * t;
        const h = Math.sqrt(x * x + y * y + z * z) - RE;
        const dR = exp(-h / 8);
        const dM = exp(-h / 1.2);
        const dO = Math.max(0, 1 - Math.abs(h - 25) / 15);
        odR += dR * dt * 0.5;
        odM += dM * dt * 0.5;
        odO += dO * dt * 0.5;
        if (!hitsGround(x, y, z, 0, sy, sz)) {
          const ls = odTo(x, y, z, 0, sy, sz, 6);
          for (let k = 0; k < 3; k++) {
            const tau = BR[k] * (odR + ls[0]) + me * (odM + ls[1]) + BO[k] * (odO + ls[2]);
            acc[k] += (BR[k] * dR * (pR + 0.018) + ms * dM * (pM + 0.01)) * exp(-tau) * dt;
          }
        }
        odR += dR * dt * 0.5;
        odM += dM * dt * 0.5;
        odO += dO * dt * 0.5;
      }
      const j = (ie * NA + ia) * 3;
      L[j] = acc[0] * ESUN;
      L[j + 1] = acc[1] * ESUN;
      L[j + 2] = acc[2] * ESUN;
    }
  }
  return L;
}
function lutAt(L: Float64Array, el: number, dAz: number, o: V3): void {
  const u = Math.sqrt(clamp(el, 0, 90) / 90) * (NE - 1);
  const v = clamp(dAz / 180, 0, 1) * (NA - 1);
  const i0 = Math.min(NE - 2, Math.floor(u));
  const j0 = Math.min(NA - 2, Math.floor(v));
  const fu = u - i0;
  const fv = v - j0;
  for (let k = 0; k < 3; k++) {
    const a = L[(i0 * NA + j0) * 3 + k];
    const b = L[(i0 * NA + j0 + 1) * 3 + k];
    const c = L[((i0 + 1) * NA + j0) * 3 + k];
    const d = L[((i0 + 1) * NA + j0 + 1) * 3 + k];
    o[k] = (a * (1 - fv) + b * fv) * (1 - fu) + (c * (1 - fv) + d * fv) * fu;
  }
}
function lightAt(h: number, el: number, mK: number): V3 {
  const ce = cos(el * DEG);
  const sy = sin(el * DEG);
  const py = RE + h;
  if (hitsGround(0, py, 0, 0, sy, ce)) return [0, 0, 0];
  return extinct(odTo(0, py, 0, 0, sy, ce, 10), mK);
}

// ---------- baked tileable 3D noise for clouds (32³, seed-independent) ----------
function h3(x: number, y: number, z: number, s: number): number {
  let h = (((Math.imul(x, 374761393) + Math.imul(y, 668265263)) | 0) + ((Math.imul(z, 1440662683) + Math.imul(s, 1274126177)) | 0)) | 0;
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  h ^= h >>> 16;
  return (h >>> 0) / 4294967296;
}
const wrap = (i: number, p: number): number => ((i % p) + p) % p;
function vnT(x: number, y: number, z: number, p: number, s: number): number {
  const xi = Math.floor(x);
  const yi = Math.floor(y);
  const zi = Math.floor(z);
  const xf = x - xi;
  const yf = y - yi;
  const zf = z - zi;
  const u = xf * xf * (3 - 2 * xf);
  const v = yf * yf * (3 - 2 * yf);
  const w = zf * zf * (3 - 2 * zf);
  const X0 = wrap(xi, p);
  const Y0 = wrap(yi, p);
  const Z0 = wrap(zi, p);
  const X1 = (X0 + 1) % p;
  const Y1 = (Y0 + 1) % p;
  const Z1 = (Z0 + 1) % p;
  const l = (a: number, b: number, t: number): number => a + (b - a) * t;
  return l(
    l(l(h3(X0, Y0, Z0, s), h3(X1, Y0, Z0, s), u), l(h3(X0, Y1, Z0, s), h3(X1, Y1, Z0, s), u), v),
    l(l(h3(X0, Y0, Z1, s), h3(X1, Y0, Z1, s), u), l(h3(X0, Y1, Z1, s), h3(X1, Y1, Z1, s), u), v),
    w,
  );
}
function worT(x: number, y: number, z: number, p: number, s: number): number {
  const xi = Math.floor(x);
  const yi = Math.floor(y);
  const zi = Math.floor(z);
  let m = 9;
  for (let k = -1; k <= 1; k++)
    for (let j = -1; j <= 1; j++)
      for (let i = -1; i <= 1; i++) {
        const cx = xi + i;
        const cy = yi + j;
        const cz = zi + k;
        const X = wrap(cx, p);
        const Y = wrap(cy, p);
        const Z = wrap(cz, p);
        const dx = cx + h3(X, Y, Z, s) - x;
        const dy = cy + h3(X, Y, Z, s + 1) - y;
        const dz = cz + h3(X, Y, Z, s + 2) - z;
        const d = dx * dx + dy * dy + dz * dz;
        if (d < m) m = d;
      }
  return Math.min(1, Math.sqrt(m));
}
let TEX: { sh: Float64Array; de: Float64Array } | null = null;
function equalise(a: Float64Array): void {
  const h = new Float64Array(2049);
  for (let i = 0; i < a.length; i++) h[Math.min(2048, Math.floor(a[i] * 2048))] += 1;
  let c = 0;
  for (let i = 0; i < 2049; i++) {
    c += h[i];
    h[i] = c / a.length;
  }
  for (let i = 0; i < a.length; i++) a[i] = h[Math.min(2048, Math.floor(a[i] * 2048))];
}
function bake(): { sh: Float64Array; de: Float64Array } {
  if (TEX) return TEX;
  const N = 32;
  const sh = new Float64Array(N * N * N);
  const de = new Float64Array(N * N * N);
  for (let z = 0; z < N; z++)
    for (let y = 0; y < N; y++)
      for (let x = 0; x < N; x++) {
        const u = x / N;
        const v = y / N;
        const w = z / N;
        let pf = 0;
        let a = 0.5;
        let t = 0;
        for (let o = 0; o < 4; o++) {
          const f = 4 << o;
          pf += a * vnT(u * f, v * f, w * f, f, 7 + o);
          t += a;
          a *= 0.5;
        }
        pf /= t;
        const wf = 1 - (worT(u * 4, v * 4, w * 4, 4, 31) * 0.625 + worT(u * 8, v * 8, w * 8, 8, 37) * 0.25 + worT(u * 16, v * 16, w * 16, 16, 41) * 0.125);
        sh[(z * N + y) * N + x] = clamp(((pf - (1 - wf)) / (1 - (1 - wf) * 0.98)) * 0.55 + wf * 0.55 - 0.1, 0, 1);
        de[(z * N + y) * N + x] = 1 - (worT(u * 4, v * 4, w * 4, 4, 53) * 0.625 + worT(u * 8, v * 8, w * 8, 8, 59) * 0.25 + worT(u * 16, v * 16, w * 16, 16, 61) * 0.125);
      }
  equalise(sh);
  equalise(de);
  TEX = { sh, de };
  return TEX;
}
function tri(T: Float64Array, x: number, y: number, z: number): number {
  const X = Math.floor(x);
  const Y = Math.floor(y);
  const Z = Math.floor(z);
  const fx = x - X;
  const fy = y - Y;
  const fz = z - Z;
  const xi = X & 31;
  const yi = Y & 31;
  const zi = Z & 31;
  const x1 = (xi + 1) & 31;
  const y1 = (yi + 1) & 31;
  const z1 = (zi + 1) & 31;
  const a = T[zi * 1024 + yi * 32 + xi];
  const b = T[zi * 1024 + yi * 32 + x1];
  const c = T[zi * 1024 + y1 * 32 + xi];
  const d = T[zi * 1024 + y1 * 32 + x1];
  const e = T[z1 * 1024 + yi * 32 + xi];
  const f = T[z1 * 1024 + yi * 32 + x1];
  const g = T[z1 * 1024 + y1 * 32 + xi];
  const h = T[z1 * 1024 + y1 * 32 + x1];
  const l0 = a + (b - a) * fx;
  const l1 = c + (d - c) * fx;
  const l2 = e + (f - e) * fx;
  const l3 = g + (h - g) * fx;
  const m0 = l0 + (l1 - l0) * fy;
  const m1 = l2 + (l3 - l2) * fy;
  return m0 + (m1 - m0) * fz;
}

const CLOUD = [
  { h0: 1.4, h1: 4.6, per: 17, sig: 34, ero: 0.36, sq: 1 },
  { h0: 1.2, h1: 2.3, per: 6, sig: 30, ero: 0.3, sq: 2.2 },
  null,
  { h0: 0.5, h1: 1.4, per: 9, sig: 7, ero: 0.45, sq: 2.2 },
];
const TYPES = ["cumulus", "stratocumulus", "cirrus", "stratus"];
const HG = (g: number, mu: number): number => (1 - g * g) / (4 * PI * pow(1 + g * g - 2 * g * mu, 1.5));
const sunElOf = (t: number): number => 52 * sin((2 * PI * (t - 6)) / 24);

export const sky: Generator = {
  id: "sky",
  title: "Sky",
  description: "A physically scattered sky at any hour: volumetric clouds, cirrus, haze, sun, a moving moon with phases, and stars.",
  aspect: 1.6,
  params: [
    { kind: "number", key: "time", label: "Time", min: 0, max: 24, step: 0.05, default: 17.6, unit: "h" },
    { kind: "choice", key: "type", label: "Clouds", default: "cumulus", options: [opt("cumulus", "Cumulus"), opt("stratocumulus", "Stratocumulus"), opt("cirrus", "Cirrus"), opt("stratus", "Stratus")] },
    { kind: "number", key: "cover", label: "Cover", min: 0, max: 1, step: 0.01, default: 0.42, unit: "%" },
    { kind: "number", key: "cirrus", label: "Cirrus veil", min: 0, max: 1, step: 0.01, default: 0.12, unit: "%" },
    { kind: "number", key: "haze", label: "Haze", min: 0, max: 1, step: 0.01, default: 0.2, unit: "%" },
    { kind: "number", key: "moon", label: "Moon phase", min: 0, max: 1, step: 0.01, default: 0.38 },
  ],
  previews: [
    { title: "Time of day", sub: "The same sky from dawn to night.", key: "time", values: [5.4, 6.6, 9, 12.5, 17.2, 18.3, 19.2, 23], labels: ["Dawn", "Sunrise", "Morning", "Noon", "Golden hour", "Sunset", "Dusk", "Night"] },
    { title: "Cloud cover", sub: "From a clear sky to overcast.", key: "cover", values: [0, 0.25, 0.5, 0.75, 1], labels: ["Clear", "Few", "Scattered", "Broken", "Overcast"] },
  ],
  render(seed, W, H, p) {
    const nz = new GNoise(stream(seed, "sky-noise"));
    const rr = stream(seed, "sky");
    const ox = rr() * 500;
    const oy = rr() * 500;
    const wind = rr() * PI;
    const az0 = (rr() - 0.5) * 40;
    const moonInc = (rr() - 0.5) * 14;
    const moonAzO = (rr() - 0.5) * 16;
    const starSeed = Math.floor(rr() * 1e9);
    const mwTilt = (rr() - 0.5) * 0.6;
    const ccx = rr() * 1000;
    const ccz = rr() * 1000;
    const shear = (rr() - 0.5) * 2;
    const time = num(p, "time");
    const typ = TYPES.indexOf(str(p, "type"));
    const haze = num(p, "haze");
    const TX = bake();
    const FOVV = 62;
    const asp = W / H;
    const tanV = sin((FOVV / 2) * DEG) / cos((FOVV / 2) * DEG);
    const tanH = tanV * asp;
    const sunEl = sunElOf(time);
    const sunAz = az0 * 0.4 + (time - 12) * 5.5;
    const bottomEl = clamp(sunEl * 0.55 - 1, -0.8, 6);
    const pitch = (FOVV / 2 + bottomEl) * DEG;
    const sinP = sin(pitch);
    const cosP = cos(pitch);
    const ce0 = cos(sunEl * DEG);
    const sx = ce0 * sin(sunAz * DEG);
    const sy = sin(sunEl * DEG);
    const sz = ce0 * cos(sunAz * DEG);
    const mK = 1 + 3.5 * haze;
    const LS = buildLUT(sunEl, mK);
    const tmp: V3 = [0, 0, 0];
    const fCyc = num(p, "moon");
    const mT = (((time - 24 * fCyc) % 24) + 24) % 24;
    const moonEl = 52 * sin((2 * PI * (mT - 6)) / 24) + moonInc;
    const moonAz = az0 * 0.4 + moonAzO + (mT - 12) * 5.5;
    const mce = cos(moonEl * DEG);
    const mdx3 = mce * sin(moonAz * DEG);
    const mdy3 = sin(moonEl * DEG);
    const mdz3 = mce * cos(moonAz * DEG);
    const mzc = mdy3 * sinP + mdz3 * cosP;
    const myc = mdy3 * cosP - mdz3 * sinP;
    const moonR = (0.26 / FOVV) * H * 2.2;
    let mx = -1e5;
    let my = -1e5;
    if (mzc > 0.05) {
      mx = (0.5 + mdx3 / mzc / (2 * tanH)) * W;
      my = (0.5 - myc / mzc / (2 * tanV)) * H;
    }
    const iAng = PI * Math.abs(1 - 2 * fCyc);
    const fracLit = (1 + cos(iAng)) / 2;
    let Lx: number;
    let Ly: number;
    let Lz: number;
    let mrot: number;
    {
      const dot = sx * mdx3 + sy * mdy3 + sz * mdz3;
      let px = sx - dot * mdx3;
      let py = sy - dot * mdy3;
      let pz = sz - dot * mdz3;
      const pl = Math.sqrt(px * px + py * py + pz * pz) || 1;
      px /= pl;
      py /= pl;
      pz /= pl;
      const hx = px;
      const hy = -(py * cosP - pz * sinP);
      const hl = Math.sqrt(hx * hx + hy * hy) || 1;
      const si = sin(iAng);
      Lx = (si * hx) / hl;
      Ly = (si * hy) / hl;
      Lz = cos(iAng);
      mrot = atan2(-mdy3 * mdx3, (1 - mdy3 * mdy3) * cosP + mdy3 * mdz3 * sinP);
    }
    const mcr = cos(mrot);
    const msr = sin(mrot);
    const mwA = 0.5 + mwTilt;
    const moonIrr = moonEl > -1 ? 3e-3 * pow(fracLit, 1.5) * ESUN : 0;
    const LM = moonIrr > 0 ? buildLUT(Math.max(moonEl, 0.5), mK) : null;
    const NIGHT: V3 = [0.9e-4 * ESUN, 1.4e-4 * ESUN, 2.6e-4 * ESUN];
    let lum = 0;
    for (const e of [15, 35, 60])
      for (const a of [30, 90, 150]) {
        lutAt(LS, e, a, tmp);
        lum += tmp[0] * 0.21 + tmp[1] * 0.72 + tmp[2] * 0.07;
        if (LM) {
          lutAt(LM, e, a, tmp);
          lum += ((tmp[0] * 0.21 + tmp[1] * 0.72 + tmp[2] * 0.07) * moonIrr) / ESUN;
        }
      }
    lum = lum / 9 + NIGHT[1];
    const expo = clamp(0.16 / lum, 0.4, 0.016 / NIGHT[1]);
    const sunL: V3[] = [];
    const moonL: V3[] = [];
    for (let h = 0; h <= 12; h++) {
      const s = lightAt(h, sunEl, mK);
      sunL.push([s[0] * ESUN, s[1] * ESUN, s[2] * ESUN]);
      if (moonIrr > 0) {
        const m = lightAt(h, Math.max(moonEl, 0), mK);
        moonL.push([m[0] * moonIrr, m[1] * moonIrr, m[2] * moonIrr]);
      } else moonL.push([0, 0, 0]);
    }
    const lightTab = (h: number, o: V3, tab: V3[]): void => {
      const f = clamp(h, 0, 11.99);
      const i = Math.floor(f);
      const t = f - i;
      for (let k = 0; k < 3; k++) o[k] = tab[i][k] + (tab[i + 1][k] - tab[i][k]) * t;
    };
    const amb: V3 = [0, 0, 0];
    for (const e of [20, 50, 80])
      for (const a of [0, 60, 120, 180]) {
        lutAt(LS, e, a, tmp);
        for (let k = 0; k < 3; k++) amb[k] += tmp[k] / 12;
        if (LM) {
          lutAt(LM, e, a, tmp);
          for (let k = 0; k < 3; k++) amb[k] += (tmp[k] * moonIrr) / ESUN / 12;
        }
      }
    for (let k = 0; k < 3; k++) amb[k] += NIGHT[k];
    const useMoon = sunEl < -7 && moonIrr > 0;
    const ldx = useMoon ? mdx3 : sx;
    const ldy = useMoon ? mdy3 : sy;
    const ldz = useMoon ? mdz3 : sz;
    const LT = useMoon ? moonL : sunL;
    const cfg = CLOUD[typ];
    const cov = typ === 2 ? 0 : num(p, "cover");
    const cirAmt = typ === 2 ? Math.max(num(p, "cover"), num(p, "cirrus")) : num(p, "cirrus");
    const vis = 60 * (1 - 0.75 * haze);
    const cw = cos(wind);
    const swd = sin(wind);
    const shz = Math.sqrt(sx * sx + sz * sz) || 1;
    const mhz = Math.sqrt(mdx3 * mdx3 + mdz3 * mdz3) || 1;
    const t2: V3 = [0, 0, 0];
    const skyCol = (dx: number, dy: number, dz: number, o: V3): void => {
      const dyc = Math.max(dy, 0.0105);
      const el = asin(dyc) / DEG;
      const hn = Math.sqrt(dx * dx + dz * dz) || 1;
      lutAt(LS, el, acos(clamp((dx * sx + dz * sz) / (hn * shz), -1, 1)) / DEG, o);
      o[0] += NIGHT[0];
      o[1] += NIGHT[1];
      o[2] += NIGHT[2];
      if (LM) {
        lutAt(LM, el, acos(clamp((dx * mdx3 + dz * mdz3) / (hn * mhz), -1, 1)) / DEG, t2);
        const k = moonIrr / ESUN;
        o[0] += t2[0] * k;
        o[1] += t2[1] * k;
        o[2] += t2[2] * k;
      }
    };
    const RAY: V3 = [0, 0, 0];
    const ray = (fx: number, fy: number): void => {
      const xc = (fx - 0.5) * 2 * tanH;
      const yc = (0.5 - fy) * 2 * tanV;
      const dyr = yc * cosP + sinP;
      const dzr = -yc * sinP + cosP;
      const n = 1 / Math.sqrt(xc * xc + dyr * dyr + dzr * dzr);
      RAY[0] = xc * n;
      RAY[1] = dyr * n;
      RAY[2] = dzr * n;
    };

    // ---- low clouds: volumetric ray march at half resolution ----
    const W2 = Math.ceil(W / 2) + 1;
    const H2 = Math.ceil(H / 2) + 1;
    const CB = new Float64Array(W2 * H2 * 4);
    for (let i = 3; i < CB.length; i += 4) CB[i] = 1;
    if (cfg && cov > 0.005) {
      const q = W >= 700 ? 1 : W >= 400 ? 0.75 : 0.5;
      const NS = round(64 * q * (typ === 3 ? 0.4 : typ === 1 ? 0.75 : 1));
      const sig = cfg.sig;
      const sc = 32 / cfg.per;
      const dsc = 32 / 1.6;
      const thick = cfg.h1 - cfg.h0;
      const lsteps = (q < 1 || typ === 3 ? [0.08, 0.3, 1] : [0.05, 0.14, 0.32, 0.7, 1.4]).map((v) => (v * thick) / 3);
      const NL = lsteps.length;
      const pc: V3 = [0, 0, 0];
      const ysc = typ === 0 ? 1.3 : 1;
      const shape = (x: number, y: number, z: number, wc: number): number => {
        const hf = (y - cfg.h0) / thick;
        if (hf < 0 || hf > 1) return 0;
        const prof = typ === 0 ? sstep(0, 0.07, hf) * sstep(1, 0.45, hf) * 0.6 + sstep(0, 0.07, hf) * 0.4 * (1 - hf) : sstep(0, 0.15, hf) * sstep(1, 0.6, hf);
        const xs = x + ccx + hf * shear;
        const zs = z + ccz;
        const rx = xs * cw + zs * swd;
        const rz = -xs * swd + zs * cw;
        const b1 = tri(TX.sh, rx * sc, y * sc * ysc, (rz * sc) / cfg.sq) * 0.68;
        if ((b1 + 0.32) * prof <= 1 - wc) return 0;
        const b = b1 + tri(TX.sh, rx * sc * 3.7 + 17, y * sc * 3.7, (rz * sc * 3.7) / cfg.sq + 5) * 0.32;
        return clamp((b * prof - (1 - wc)) / Math.max(wc, 0.05), 0, 1);
      };
      for (let j = 0; j < H2; j++)
        for (let i = 0; i < W2; i++) {
          ray(Math.min(1, (i * 2) / W), Math.min(1, (j * 2) / H));
          const dx = RAY[0];
          const dy = RAY[1];
          const dz = RAY[2];
          const o = (j * W2 + i) * 4;
          if (dy < 0.004) continue;
          const oyy = RE + 0.02;
          const t0 = sphT(0, oyy, 0, dx, dy, dz, RE + cfg.h0);
          const t1 = sphT(0, oyy, 0, dx, dy, dz, RE + cfg.h1);
          if (t0 < 0 || t0 > 140) continue;
          const te = Math.min(t1, t0 + (typ === 0 ? 16 : typ === 3 ? 7 : 12));
          const mu = dx * ldx + dy * ldy + dz * ldz;
          const ph0 = HG(0.75, mu) * 0.7 + HG(-0.25, mu) * 0.3;
          const ph1 = HG(0.4, mu) * 0.7 + HG(-0.15, mu) * 0.3;
          const ph2 = 1 / (4 * PI);
          let T = 1;
          let r = 0;
          let g = 0;
          let b = 0;
          let tw = 0;
          let ww = 0;
          let miss = 0;
          let tauC = -1;
          let hits = 0;
          const dt = (te - t0) / NS;
          let t = t0 + dt * (0.25 + 0.5 * hash2(i, j, 77));
          const wc = clamp(cov + (nz.fbm((dx * (t0 + te)) / 2 / 38 + ox, (dz * (t0 + te)) / 2 / 38 + oy, 3) - 0.5) * 0.55 * (1 - cov * 0.6), 0, 1);
          for (let s = 0; s < NS && T > 0.012; s++, t += dt) {
            const x = dx * t;
            const z = dz * t;
            const yy = oyy + dy * t;
            const y = Math.sqrt(x * x + yy * yy + z * z) - RE;
            let d = shape(x, y, z, wc);
            if (d <= 0) {
              if (miss++ > 1) {
                t += dt;
                s++;
              }
              continue;
            }
            miss = 0;
            const hf = (y - cfg.h0) / thick;
            const det = tri(TX.de, x * dsc, y * dsc, z * dsc);
            d = clamp((d - det * cfg.ero * (1 - d)) / (1 - det * cfg.ero * 0.5), 0, 1);
            if (d <= 0.002) continue;
            const sg = d * sig;
            let tau = tauC;
            if (tau < 0 || (hits++ & 1) === 0) {
              tau = 0;
              for (let k = 0; k < NL; k++) {
                const l = lsteps[k];
                const dl = k > 0 ? lsteps[k] - lsteps[k - 1] : lsteps[0];
                tau += shape(x + ldx * l, y + ldy * l, z + ldz * l, wc) * sig * dl;
              }
              tauC = tau;
            }
            lightTab(y, pc, LT);
            const E = exp(-tau) * ph0 + 0.5 * exp(-tau * 0.25) * ph1 + 0.25 * exp(-tau * 0.06) * ph2;
            const powder = 1 - exp(-sg * 0.35);
            const lk = E * (0.5 + 0.5 * powder);
            const ak = (0.25 + 0.75 * hf) * (0.6 + 0.4 * powder) * 1.15;
            const a = 1 - exp(-sg * dt);
            r += T * (pc[0] * lk + amb[0] * ak) * a;
            g += T * (pc[1] * lk + amb[1] * ak) * a;
            b += T * (pc[2] * lk + amb[2] * ak) * a;
            tw += T * a * t;
            ww += T * a;
            T *= 1 - a;
          }
          if (ww > 0) {
            skyCol(dx, dy, dz, tmp);
            const fog = exp(-tw / ww / vis);
            const cvr = 1 - T;
            r = r * fog + tmp[0] * (1 - fog) * cvr;
            g = g * fog + tmp[1] * (1 - fog) * cvr;
            b = b * fog + tmp[2] * (1 - fog) * cvr;
          }
          CB[o] = r;
          CB[o + 1] = g;
          CB[o + 2] = b;
          CB[o + 3] = T;
        }
      const C2 = new Float64Array(CB.length);
      for (let j = 0; j < H2; j++)
        for (let i = 0; i < W2; i++)
          for (let k = 0; k < 4; k++) {
            let s = 0;
            let w = 0;
            for (let bb = -1; bb <= 1; bb++)
              for (let aa = -1; aa <= 1; aa++) {
                const ii = i + aa;
                const jj = j + bb;
                if (ii < 0 || jj < 0 || ii >= W2 || jj >= H2) continue;
                const q2 = (2 - Math.abs(aa)) * (2 - Math.abs(bb));
                s += CB[(jj * W2 + ii) * 4 + k] * q2;
                w += q2;
              }
            C2[(j * W2 + i) * 4 + k] = s / w;
          }
      CB.set(C2);
    }

    // ---- sky and cirrus at half resolution ----
    const SB = new Float64Array(W2 * H2 * 3);
    const CT = new Float64Array(W2 * H2);
    const night = sstep(-4, -14, sunEl);
    const t3: V3 = [0, 0, 0];
    for (let j = 0; j < H2; j++)
      for (let i = 0; i < W2; i++) {
        ray(Math.min(1, (i * 2) / W), Math.min(1, (j * 2) / H));
        const dx = RAY[0];
        const dy = RAY[1];
        const dz = RAY[2];
        skyCol(dx, dy, dz, tmp);
        let R = tmp[0];
        let G = tmp[1];
        let B = tmp[2];
        const el = asin(Math.max(dy, 0.0105)) / DEG;
        const hg = exp(-el / 6) * night;
        R += hg * NIGHT[0] * 1.6;
        G += hg * NIGHT[1] * 1.3;
        B += hg * NIGHT[2] * 0.9;
        let cT = 1;
        if (cirAmt > 0.02 && dy > 0.01) {
          const t = sphT(0, RE + 0.02, 0, dx, dy, dz, RE + 9);
          if (t > 0 && t < 400) {
            const u = (dx * t) / 14;
            const v = (dz * t) / 14;
            const ru = u * cw + v * swd;
            const rv = -u * swd + v * cw;
            const qq = nz.fbm(ru * 0.35 + ox, rv * 0.9 + oy, 3);
            let d = nz.fbm(ru * 0.3 + (qq - 0.5) * 1.5 + ox, rv * 1.3 + (qq - 0.5) * 0.5 + oy, 6);
            d = clamp((d - (1 - 0.8 * cirAmt)) * 2.6, 0, 1) * (0.6 + 0.4 * nz.fbm(ru * 3, rv * 7, 3));
            const tau = d * 0.55 * Math.min(1, cirAmt * 2);
            if (tau > 0.002) {
              cT = exp(-tau);
              const mu = dx * ldx + dy * ldy + dz * ldz;
              lightTab(9, t3, LT);
              const ph = HG(0.7, mu) * 0.7 + HG(0.1, mu) * 0.3;
              const a = 1 - cT;
              const fog = exp(-t / (vis * 3));
              R = R * cT + ((t3[0] * ph * 1.2 + amb[0] * 0.9) * fog + tmp[0] * (1 - fog)) * a;
              G = G * cT + ((t3[1] * ph * 1.2 + amb[1] * 0.9) * fog + tmp[1] * (1 - fog)) * a;
              B = B * cT + ((t3[2] * ph * 1.2 + amb[2] * 0.9) * fog + tmp[2] * (1 - fog)) * a;
            }
          }
        }
        const o = j * W2 + i;
        SB[o * 3] = R;
        SB[o * 3 + 1] = G;
        SB[o * 3 + 2] = B;
        CT[o] = cT;
      }

    // ---- full resolution: sun, stars, moon, then composite ----
    const TMN = 4096;
    const TML = new Float64Array(TMN + 1);
    for (let i = 0; i <= TMN; i++) {
      const c = (i / TMN) * (i / TMN) * 24;
      const v = (c * (2.51 * c + 0.03)) / (c * (2.43 * c + 0.59) + 0.14);
      TML[i] = pow(clamp(v, 0, 1), 1 / 2.2) * 255;
    }
    const tmg = (c: number): number => {
      c *= expo;
      if (c <= 0) return 0;
      const f = Math.sqrt(Math.min(c, 24) / 24) * TMN;
      const i = Math.floor(f);
      return i >= TMN ? TML[TMN] : TML[i] + (TML[i + 1] - TML[i]) * (f - i);
    };
    const sunR = 0.27 * DEG * 2.2;
    const cosSun = cos(sunR);
    const sunDisc: V3 = [0, 0, 0];
    lightTab(0, sunDisc, sunL);
    const sunRad = 1 / (PI * sunR * sunR);
    const mvis = extinct(odTo(0, RE + 0.05, 0, mdx3, Math.max(mdy3, 0.01), mdz3, 8), mK);
    const Lmn = 0.03 * ESUN * 0.05;
    const d = newImage(W, H);
    for (let y = 0; y < H; y++) {
      const fy = y / H;
      for (let x = 0; x < W; x++) {
        const fx = x / W;
        ray(fx, fy);
        const dx = RAY[0];
        const dy = RAY[1];
        const dz = RAY[2];
        const dyc = Math.max(dy, 0.0105);
        let bR = 0;
        let bG = 0;
        let bB = 0;
        const cs = dx * sx + dyc * sy + dz * sz;
        if (sunEl > -1.5 && cs > 0.95) {
          const ang = acos(clamp(cs, -1, 1));
          if (cs > cosSun - 0.00002) {
            const q = ang / sunR;
            const ld = 0.4 + 0.6 * Math.sqrt(Math.max(0, 1 - q * q));
            const cv = clamp((sunR - ang) / (sunR * 0.15), 0, 1) * ld * sunRad;
            bR += sunDisc[0] * cv;
            bG += sunDisc[1] * cv;
            bB += sunDisc[2] * cv;
          }
          const gl = (0.9 * exp(-ang / (0.5 * DEG)) + 0.1 * exp(-ang / (2.5 * DEG))) * 0.12;
          bR += sunDisc[0] * gl;
          bG += sunDisc[1] * gl;
          bB += sunDisc[2] * gl;
        }
        if (night > 0.01) {
          const sk = (0.9 / expo) * night * sstep(0, 0.1, dy);
          const h1 = hash2(x, y, starSeed);
          let st = 0;
          let tint = 0.5;
          if (h1 > 0.9988) st += pow((h1 - 0.9988) / 0.0012, 2.4) * 0.45 + 0.03;
          const cx = Math.floor(x / 3);
          const cy = Math.floor(y / 3);
          const h2 = hash2(cx, cy, starSeed + 77);
          if (h2 > 0.9855) {
            const jx = hash2(cx, cy, 5) * 2 + 0.5;
            const jy = hash2(cx, cy, 9) * 2 + 0.5;
            const ddx = x - cx * 3 - jx;
            const ddy = y - cy * 3 - jy;
            st += (0.3 + ((h2 - 0.9855) / 0.0145) * 1.3) * exp(-(ddx * ddx + ddy * ddy) / 0.7);
            tint = hash2(cx, cy, 3);
          }
          const wy = fy - (0.16 + mwA * fx * 0.5 + 0.12);
          const w = exp(-(wy * wy) / 0.02) * Math.max(0, 0.15 + 1.1 * (nz.fbm(fx * 6 + ox, fy * 6, 5) - 0.35)) * 0.08;
          const ex = exp(-0.25 / Math.max(dyc, 0.03));
          bR += (st * (0.8 + 0.4 * tint) + w * 0.9) * sk * ex;
          bG += (st * 0.9 + w) * sk * ex;
          bB += (st * (1.15 - 0.4 * tint) + w * 1.3) * sk * ex;
        }
        if (mx > -1e4 && moonEl > -1) {
          const mdx = x - mx;
          const mdy = y - my;
          const mr = Math.sqrt(mdx * mdx + mdy * mdy);
          if (mr < moonR * 12) {
            const halo = (exp(-mr / (moonR * 1.4)) * 0.02 + exp(-mr / (moonR * 5)) * 0.006) * pow(fracLit, 1.6) * Lmn;
            bR += halo * mvis[0];
            bG += halo * mvis[1];
            bB += halo * mvis[2];
            if (mr < moonR + 1.5) {
              const c = clamp((moonR + 0.7 - mr) / 1.4, 0, 1);
              const nx = mdx / moonR;
              const ny = mdy / moonR;
              const nzz = Math.sqrt(Math.max(0, 1 - nx * nx - ny * ny));
              const lit = sstep(-0.04, 0.06, nx * Lx + ny * Ly + nzz * Lz);
              const rx = nx * mcr - ny * msr;
              const ry = nx * msr + ny * mcr;
              const maria = 0.7 + 0.3 * sstep(0.35, 0.65, nz.fbm(rx * 2.2 + 4, ry * 2.2 + 9, 4));
              const crat = 0.93 + 0.07 * nz.n2(rx * 14, ry * 14);
              const bb = (lit * maria * crat * (0.9 + 0.2 * nzz) + 0.012 * (1 - lit)) * Lmn * c;
              bR += bb * mvis[0];
              bG += bb * mvis[1];
              bB += bb * mvis[2];
            }
          }
        }
        const gx = x / 2;
        const gy = y / 2;
        const i0 = Math.floor(gx);
        const j0 = Math.floor(gy);
        const ax = gx - i0;
        const ay = gy - j0;
        let R = 0;
        let G = 0;
        let B = 0;
        let cr = 0;
        let cg = 0;
        let cb = 0;
        let ct = 0;
        let cT = 0;
        for (let k = 0; k < 4; k++) {
          const ii = Math.min(i0 + (k & 1), W2 - 1);
          const jj = Math.min(j0 + (k >> 1), H2 - 1);
          const w = ((k & 1) !== 0 ? ax : 1 - ax) * ((k >> 1) !== 0 ? ay : 1 - ay);
          const q = jj * W2 + ii;
          cr += CB[q * 4] * w;
          cg += CB[q * 4 + 1] * w;
          cb += CB[q * 4 + 2] * w;
          ct += CB[q * 4 + 3] * w;
          R += SB[q * 3] * w;
          G += SB[q * 3 + 1] * w;
          B += SB[q * 3 + 2] * w;
          cT += CT[q] * w;
        }
        const ct3 = ct * ct * ct * cT;
        R = R * ct + bR * ct3 + cr;
        G = G * ct + bG * ct3 + cg;
        B = B * ct + bB * ct3 + cb;
        const dn = hash2(x, y, 991) - 0.5;
        put(d, y * W + x, tmg(R) + dn, tmg(G) + dn, tmg(B) + dn);
      }
    }
    return d;
  },
};
