/**
 * Shared building blocks of the generators. Everything here is deterministic: only `+ - * /`,
 * `floor`, `sqrt`, integer ops and the functions from `dmath`. `crates/swirlseed/src/gen/core.rs`
 * mirrors this file expression by expression.
 */
import { PI, TAU, cbrt, cos, exp, ln, pow, sin } from "../dmath.js";
import { linearToSrgb255, oklabToLinear } from "../color.js";
import { makeRng } from "../rng.js";

export { PI, TAU };
export const DEG = PI / 180;

export const clamp = (x: number, a: number, b: number): number => (x < a ? a : x > b ? b : x);
export const lerp = (a: number, b: number, t: number): number => a + (b - a) * t;
export function sstep(a: number, b: number, x: number): number {
  const t = clamp((x - a) / (b - a), 0, 1);
  return t * t * (3 - 2 * t);
}
export const hypot2 = (x: number, y: number): number => Math.sqrt(x * x + y * y);
export const hypot3 = (x: number, y: number, z: number): number => Math.sqrt(x * x + y * y + z * z);
/** Round half up, like `Math.round`. */
export const round = (x: number): number => Math.floor(x + 0.5);
/** Float (0…255) to a byte, rounding half up. */
export function toByte(v: number): number {
  if (!(v > 0)) return 0;
  if (v >= 255) return 255;
  return Math.floor(v + 0.5);
}

/** A random stream in [0, 1) for one aspect of a generator. */
export type Rand = () => number;
export function stream(seed: string, salt: string): Rand {
  const r = makeRng(seed, "gen/" + salt);
  return () => r.next();
}
/** A 20-bit integer derived from a stream, used to offset noise. */
export const seedInt = (R: Rand): number => Math.floor(R() * 1e6);

/** Integer lattice hash in [0, 1). Arguments are truncated to int32 like `x | 0`. */
export function hash2(x: number, y: number, s: number): number {
  let h = (Math.imul(x | 0, 374761393) + Math.imul(y | 0, 668265263) + Math.imul(s | 0, 1274126177)) | 0;
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  h ^= h >>> 16;
  return (h >>> 0) / 4294967296;
}
export function hash3(x: number, y: number, z: number, s: number): number {
  let h =
    (((Math.imul(x | 0, 374761393) + Math.imul(y | 0, 668265263)) | 0) +
      ((Math.imul(z | 0, 1440662683) + Math.imul(s | 0, 1274126177)) | 0)) |
    0;
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  h ^= h >>> 16;
  return (h >>> 0) / 4294967296;
}

/** Value noise in [0, 1). */
export function vnoise(x: number, y: number, s: number): number {
  const xi = Math.floor(x);
  const yi = Math.floor(y);
  const xf = x - xi;
  const yf = y - yi;
  const u = xf * xf * (3 - 2 * xf);
  const v = yf * yf * (3 - 2 * yf);
  const a = hash2(xi, yi, s);
  const b = hash2(xi + 1, yi, s);
  const c = hash2(xi, yi + 1, s);
  const d = hash2(xi + 1, yi + 1, s);
  return a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v;
}
/** Fractal value noise with a rotation between octaves. */
export function fbm2(x: number, y: number, s: number, o: number): number {
  let t = 0;
  let a = 0.5;
  let n = 0;
  for (let i = 0; i < o; i++) {
    t += a * vnoise(x, y, s + i * 31);
    n += a;
    a *= 0.5;
    const nx = (x * 0.8 + y * 0.6) * 2.03 + 11.7;
    const ny = (-x * 0.6 + y * 0.8) * 2.03 + 5.3;
    x = nx;
    y = ny;
  }
  return t / n;
}
/** Fractal value noise, plain scaling between octaves (used by the marbling papers). */
export function fbmS(x: number, y: number, s: number, o: number): number {
  let t = 0;
  let a = 0.5;
  let n = 0;
  for (let i = 0; i < o; i++) {
    t += a * vnoise(x, y, s + i * 31);
    n += a;
    a *= 0.5;
    x *= 2.03;
    y *= 2.03;
  }
  return t / n;
}
export function vnoise3(x: number, y: number, z: number, s: number): number {
  const xi = Math.floor(x);
  const yi = Math.floor(y);
  const zi = Math.floor(z);
  const xf = x - xi;
  const yf = y - yi;
  const zf = z - zi;
  const u = xf * xf * (3 - 2 * xf);
  const v = yf * yf * (3 - 2 * yf);
  const w = zf * zf * (3 - 2 * zf);
  const a = hash3(xi, yi, zi, s);
  const b = hash3(xi + 1, yi, zi, s);
  const c = hash3(xi, yi + 1, zi, s);
  const d = hash3(xi + 1, yi + 1, zi, s);
  const e = hash3(xi, yi, zi + 1, s);
  const f = hash3(xi + 1, yi, zi + 1, s);
  const g = hash3(xi, yi + 1, zi + 1, s);
  const h = hash3(xi + 1, yi + 1, zi + 1, s);
  const x1 = a + (b - a) * u;
  const x2 = c + (d - c) * u;
  const x3 = e + (f - e) * u;
  const x4 = g + (h - g) * u;
  const y1 = x1 + (x2 - x1) * v;
  const y2 = x3 + (x4 - x3) * v;
  return y1 + (y2 - y1) * w;
}
export function fbm3(x: number, y: number, z: number, s: number, o: number): number {
  let t = 0;
  let a = 0.5;
  let n = 0;
  for (let i = 0; i < o; i++) {
    t += a * vnoise3(x, y, z, s + i * 31);
    n += a;
    a *= 0.5;
    const nx = y * 2.03 + 17.1;
    const ny = z * 2.03 + 3.7;
    const nz = x * 2.03 + 9.3;
    x = nx;
    y = ny;
    z = nz;
  }
  return t / n;
}

// ---------- colour ----------
export type V3 = [number, number, number];
export const oklab2lin = (L: number, a: number, b: number): V3 => oklabToLinear(L, a, b);

const ENC_N = 4096;
const ENC = ((): Float64Array => {
  const t = new Float64Array(ENC_N + 1);
  for (let i = 0; i <= ENC_N; i++) t[i] = linearToSrgb255(i / ENC_N);
  return t;
})();
/** Linear light (0…1) to sRGB 0…255 through a fine table (exact same table in Rust). */
export function enc(v: number): number {
  if (!(v > 0)) return 0;
  if (v >= 1) return 255;
  const f = v * ENC_N;
  const i = Math.floor(f);
  return ENC[i] + (ENC[i + 1] - ENC[i]) * (f - i);
}
/** sRGB byte to linear light. */
export const lin = (c: number): number => pow(c / 255, 2.2);
export const lin3 = (c: V3): V3 => [lin(c[0]), lin(c[1]), lin(c[2])];
export function hexRgb(h: string): V3 {
  const v = parseInt(h.slice(1), 16);
  return [(v >> 16) & 255, (v >> 8) & 255, v & 255];
}
/** sRGB bytes to OKLab. */
export function srgbToLab(c: V3): V3 {
  const l = [lin(c[0]), lin(c[1]), lin(c[2])];
  const L_ = cbrt(0.4122214708 * l[0] + 0.5363325363 * l[1] + 0.0514459929 * l[2]);
  const M_ = cbrt(0.2119034982 * l[0] + 0.6806995451 * l[1] + 0.1073969566 * l[2]);
  const S_ = cbrt(0.0883024619 * l[0] + 0.2817188376 * l[1] + 0.6299787005 * l[2]);
  return [
    0.2104542553 * L_ + 0.793617785 * M_ - 0.0040720468 * S_,
    1.9779984951 * L_ - 2.428592205 * M_ + 0.4505937099 * S_,
    0.0259040371 * L_ + 0.7827717662 * M_ - 0.808675766 * S_,
  ];
}
/** OKLCH → OKLab, shrinking chroma until the colour fits (hue in degrees). */
export function oklchFit(L: number, C: number, h: number): V3 {
  const hr = h * DEG;
  const ch = cos(hr);
  const sh = sin(hr);
  let c = C;
  for (let i = 0; i < 14; i++) {
    const l = oklab2lin(L, c * ch, c * sh);
    if (l[0] >= -0.001 && l[0] <= 1.001 && l[1] >= -0.001 && l[1] <= 1.001 && l[2] >= -0.001 && l[2] <= 1.001) break;
    c *= 0.88;
  }
  return [L, c * ch, c * sh];
}

// ---------- images ----------
/** Output image: RGBA bytes. */
export function newImage(W: number, H: number): Uint8ClampedArray {
  return new Uint8ClampedArray(W * H * 4);
}
export function put(d: Uint8ClampedArray, i: number, r: number, g: number, b: number): void {
  const o = i * 4;
  d[o] = toByte(r);
  d[o + 1] = toByte(g);
  d[o + 2] = toByte(b);
  d[o + 3] = 255;
}

/** Separable box blur on doubles, clamped at the edges. */
export function blur(a: Float64Array, W: number, H: number, r: number): Float64Array {
  if (r < 1) return a;
  const t = new Float64Array(W * H);
  const o = new Float64Array(W * H);
  const n = 2 * r + 1;
  for (let y = 0; y < H; y++) {
    let s = 0;
    const row = y * W;
    for (let x = -r; x <= r; x++) s += a[row + clamp(x, 0, W - 1)];
    for (let x = 0; x < W; x++) {
      t[row + x] = s / n;
      s += a[row + Math.min(W - 1, x + r + 1)] - a[row + Math.max(0, x - r)];
    }
  }
  for (let x = 0; x < W; x++) {
    let s = 0;
    for (let y = -r; y <= r; y++) s += t[clamp(y, 0, H - 1) * W + x];
    for (let y = 0; y < H; y++) {
      o[y * W + x] = s / n;
      s += t[Math.min(H - 1, y + r + 1) * W + x] - t[Math.max(0, y - r) * W + x];
    }
  }
  return o;
}

// ---------- paper and pigment (watercolour family) ----------
export function paperHeight(W: number, H: number, S: number, scale: number): Float64Array {
  const h = new Float64Array(W * H);
  const m = Math.min(W, H) / scale;
  for (let y = 0; y < H; y++)
    for (let x = 0; x < W; x++) {
      const u = x / m;
      const v = y / m;
      h[y * W + x] = 0.5 * fbm2(u * 26, v * 26, S + 101, 3) + 0.3 * fbm2(u * 70, v * 70, S + 102, 2) + 0.2 * hash2(x, y, S + 103);
    }
  return h;
}
export function paperLight(h: Float64Array, W: number, H: number, k: number): Float64Array {
  const L = new Float64Array(W * H);
  for (let y = 0; y < H; y++)
    for (let x = 0; x < W; x++) {
      const i = y * W + x;
      const xm = x > 0 ? h[i - 1] : h[i];
      const ym = y > 0 ? h[i - W] : h[i];
      L[i] = 1 + ((h[i] - xm) * 0.7 + (h[i] - ym) * 0.7) * k;
    }
  return L;
}
export interface PigmentOpts {
  S: number;
  ph: Float64Array | null;
  gran: number;
  edge: number;
  edgeR: number;
  mottleF: number;
  bloom: number;
  bloomF: number;
  bloomT: number;
  strength: number;
}
/** One layer of transparent pigment: A is coverage (0…1); absorbance is added into Ab (rgb interleaved). */
export function pigmentLayer(Ab: Float64Array, A: Float64Array, W: number, H: number, pig: V3, o: PigmentOpts): void {
  const m = Math.min(W, H);
  const S = o.S;
  const k = [-ln(clamp(pig[0], 0.02, 0.999)), -ln(clamp(pig[1], 0.02, 0.999)), -ln(clamp(pig[2], 0.02, 0.999))];
  const er = Math.max(1, round(m * o.edgeR));
  const Ab1 = blur(blur(A, W, H, er), W, H, er);
  const fq = o.mottleF;
  for (let y = 0; y < H; y++)
    for (let x = 0; x < W; x++) {
      const i = y * W + x;
      const a = A[i];
      if (a < 0.003) continue;
      const u = x / m;
      const v = y / m;
      const edge = Math.max(0, a - Ab1[i]);
      const mot = 0.8 + 0.4 * fbm2(u * fq + S * 0.001, v * fq, S + 11, 4);
      let D = a * mot + edge * o.edge;
      if (o.bloom > 0) {
        const n = fbm2(u * o.bloomF, v * o.bloomF, S + 21, 5);
        const t = o.bloomT;
        const b = sstep(t, t + 0.015, n);
        D *= 1 - 0.3 * b * a;
        const q = (n - t) / 0.007;
        D += a * 0.55 * o.bloom * exp(-q * q);
      }
      const g = o.ph ? 1 - o.gran + o.gran * 2 * o.ph[i] * (0.6 + 0.4 * fbm2(u * 40, v * 40, S + 31, 2)) : 1;
      D *= g * o.strength;
      Ab[i * 3] += D * k[0];
      Ab[i * 3 + 1] += D * k[1];
      Ab[i * 3 + 2] += D * k[2];
    }
}
export function developPaper(Ab: Float64Array, W: number, H: number, paper: V3, pl: Float64Array): Uint8ClampedArray {
  const d = newImage(W, H);
  for (let i = 0; i < W * H; i++) {
    const l = pl[i];
    put(d, i, enc(paper[0] * exp(-Ab[i * 3]) * l), enc(paper[1] * exp(-Ab[i * 3 + 1]) * l), enc(paper[2] * exp(-Ab[i * 3 + 2]) * l));
  }
  return d;
}
