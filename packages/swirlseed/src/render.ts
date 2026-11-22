import { TAU, cos, exp, pow, roundHalfUp, sin } from "./dmath.js";
import { labToSrgb255 } from "./color.js";
import { applyHalftone } from "./halftone.js";
import { BAYER8, Noise, boxBlur, hash01 } from "./noise.js";
import type { RenderOptions } from "./options.js";
import { type PatternParams, type Stop, createPattern } from "./params.js";

const clamp01 = (v: number): number => (v < 0 ? 0 : v > 1 ? 1 : v);

function buildLut(stops: Stop[], size: number): Float32Array {
  const s = [...stops].sort((p, q) => p.pos - q.pos);
  const lut = new Float32Array(size * 3);
  for (let i = 0; i < size; i++) {
    const t = i / (size - 1);
    let L: number;
    let a: number;
    let b: number;
    if (t <= s[0].pos) {
      ({ L, a, b } = s[0]);
    } else if (t >= s[s.length - 1].pos) {
      ({ L, a, b } = s[s.length - 1]);
    } else {
      let k = 0;
      while (t > s[k + 1].pos) k++;
      const p0 = s[k];
      const p1 = s[k + 1];
      let f = (t - p0.pos) / (p1.pos - p0.pos || 1);
      f = f * f * (3 - 2 * f); // smoothstep between neighbouring stops
      L = p0.L + (p1.L - p0.L) * f;
      a = p0.a + (p1.a - p0.a) * f;
      b = p0.b + (p1.b - p0.b) * f;
    }
    const [r, g, bl] = labToSrgb255({ L, a, b });
    lut[i * 3] = r;
    lut[i * 3 + 1] = g;
    lut[i * 3 + 2] = bl;
  }
  return lut;
}

/**
 * Render a pattern to RGBA (`width * height * 4` bytes).
 * The base field is smooth, so 512×512 is plenty; textures are computed in real pixels.
 */
export function renderPattern(
  p: PatternParams,
  width: number,
  height: number,
  o: RenderOptions = {},
): Uint8ClampedArray {
  const noise = new Noise(p.perm);

  const N = width * height;
  const minSide = Math.min(width, height);
  const scale = minSide / 512; // texture scale: same look at any resolution
  const aspect = width / height;
  const ca = cos(p.angle);
  const sa = sin(p.angle);
  const invStretch = 1 / p.stretch;
  const { cx, cy, strength, radius } = p.swirl;
  const rad2 = radius * radius;

  // Reeded glass setup
  const ribs = o.ribs ?? null;
  let rc = 1;
  let rs = 0;
  let rw = 0;
  let rStr = 0;
  let rShade = 0;
  if (ribs) {
    const a = ribs.angle ?? 0;
    rc = cos(a);
    rs = sin(a);
    rw = 1 / Math.max(1, ribs.count ?? 24);
    rStr = ribs.strength ?? 0.6;
    rShade = ribs.shade ?? 0.5;
  }
  const ribFr = ribs ? new Float32Array(N) : null;

  // Pass 1: scalar field
  const field = new Float32Array(N);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      let X = ((x + 0.5) / width - 0.5) * aspect;
      let Y = (y + 0.5) / height - 0.5;

      if (ribFr) {
        const S = X * rc + Y * rs;
        const k = S / rw;
        const fr = k - Math.floor(k) - 0.5; // -0.5…0.5 inside a flute
        const d = -fr * rw * 2 * rStr;
        X += d * rc;
        Y += d * rs;
        ribFr[y * width + x] = fr;
      }

      const dx = X - cx;
      const dy = Y - cy;
      const ang = strength * exp(-(dx * dx + dy * dy) / rad2);
      const c = cos(ang);
      const s = sin(ang);
      X = cx + dx * c - dy * s;
      Y = cy + dx * s + dy * c;

      let u = (X * ca + Y * sa) * p.baseFreq * invStretch;
      let v = (-X * sa + Y * ca) * p.baseFreq;

      for (const l of p.layers) {
        const wx = noise.fbm(u * l.freq + l.ox, v * l.freq + l.oy, 3);
        const wy = noise.fbm(u * l.freq + l.ox2, v * l.freq + l.oy2, 3);
        u += l.amp * wx;
        v += l.amp * wy;
      }

      field[y * width + x] = v + p.fbmWeight * noise.fbm(u * 0.8 + 31.7, v * 0.8 - 12.3, 3);
    }
  }

  // Normalise by the 2nd–98th percentile, sampled on a 2D grid at the same normalised points
  // for every frame size, so the look does not depend on the resolution.
  const G = 96;
  const sample: number[] = [];
  for (let gy = 0; gy < G; gy++) {
    const sy = Math.floor(((gy + 0.5) * height) / G);
    for (let gx = 0; gx < G; gx++) sample.push(field[sy * width + Math.floor(((gx + 0.5) * width) / G)]);
  }
  sample.sort((a, b) => a - b);
  const lo = sample[Math.floor(sample.length * 0.02)];
  const hi = sample[Math.floor(sample.length * 0.98)];
  const inv = 1 / (hi - lo || 1);

  const tm = new Float32Array(N);
  for (let i = 0; i < N; i++) {
    let t = clamp01((field[i] - lo) * inv);
    if (p.mode === "bands") t = 0.5 - 0.5 * cos(TAU * (t * p.repeats + p.phase));
    tm[i] = pow(t, p.gamma);
  }

  // Pass 2: palette + lighting → float RGB (0…255)
  const LUT_N = 2048;
  const lut = buildLut(p.stops, LUT_N);
  const rgb = new Float32Array(N * 3);

  const shine = o.shine ?? p.shine;
  // Height map for lighting: a smoothed copy removes creases on plateaus and flute edges.
  let hm: Float32Array = tm;
  if (shine > 0) {
    const br = Math.max(1, roundHalfUp(1.8 * scale));
    hm = boxBlur(boxBlur(tm, width, height, br), width, height, br);
  }
  // light from the top-left, plus the half vector for the specular highlight
  const Llen = Math.sqrt(-0.45 * -0.45 + -0.6 * -0.6 + 0.66 * 0.66);
  const Lx = -0.45 / Llen;
  const Ly = -0.6 / Llen;
  const Lz = 0.66 / Llen;
  const Hlen = Math.sqrt(Lx * Lx + Ly * Ly + (Lz + 1) * (Lz + 1));
  const Hx = Lx / Hlen;
  const Hy = Ly / Hlen;
  const Hz = (Lz + 1) / Hlen;
  let flatSpec = Hz; // Hz^64
  for (let i = 0; i < 6; i++) flatSpec *= flatSpec;
  const slopeK = 0.35 * minSide; // derivative per frame unit, not per pixel

  for (let y = 0; y < height; y++) {
    const ym = y > 0 ? y - 1 : y;
    const yp = y < height - 1 ? y + 1 : y;
    for (let x = 0; x < width; x++) {
      const i = y * width + x;
      const li = roundHalfUp(tm[i] * (LUT_N - 1)) * 3;
      let r = lut[li];
      let g = lut[li + 1];
      let b = lut[li + 2];

      if (shine > 0) {
        const xm = x > 0 ? x - 1 : x;
        const xp = x < width - 1 ? x + 1 : x;
        const gx = ((hm[y * width + xp] - hm[y * width + xm]) / (xp - xm || 1)) * slopeK;
        const gy = ((hm[yp * width + x] - hm[ym * width + x]) / (yp - ym || 1)) * slopeK;
        const il = 1 / Math.sqrt(gx * gx + gy * gy + 1);
        const nx = -gx * il;
        const ny = -gy * il;
        const nz = il;
        const diff = nx * Lx + ny * Ly + nz * Lz - Lz; // 0 on a flat surface
        let sp = clamp01(nx * Hx + ny * Hy + nz * Hz);
        for (let q = 0; q < 6; q++) sp *= sp; // ^64
        const spec = Math.max(0, sp - flatSpec);
        const k = Math.max(0.45, Math.min(1.6, 1 + shine * diff * 1.4));
        const add = shine * spec * 230;
        r = r * k + add;
        g = g * k + add;
        b = b * k + add;
      }

      if (ribFr) {
        const k = 1 + rShade * ribFr[i] * 0.6;
        r *= k;
        g *= k;
        b *= k;
      }

      const o3 = i * 3;
      rgb[o3] = r;
      rgb[o3 + 1] = g;
      rgb[o3 + 2] = b;
    }
  }

  // Pass 3: halftone
  if (o.halftone) applyHalftone(rgb, width, height, scale, o.halftone, p.stops);

  // Pass 4: grain
  if (o.grain) {
    const amount = (o.grain.amount ?? 0.35) * 70;
    const size = Math.max(1, roundHalfUp((o.grain.size ?? 1) * scale));
    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        const n = hash01(Math.floor(x / size), Math.floor(y / size), 7) - 0.5;
        const o3 = (y * width + x) * 3;
        rgb[o3] += n * amount;
        rgb[o3 + 1] += n * amount;
        rgb[o3 + 2] += n * amount;
      }
    }
  }

  // Pass 5: quantise (Bayer dithering, or a faint dither that hides banding)
  const out = new Uint8ClampedArray(N * 4);
  const dz = o.dither ?? null;
  const levels = dz ? Math.min(32, Math.max(2, roundHalfUp(dz.levels ?? 4))) : 256;
  const dScale = Math.max(1, roundHalfUp((dz?.scale ?? 1.5) * scale));
  const dAmount = dz?.amount ?? 1;
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const i = y * width + x;
      const o3 = i * 3;
      const o4 = i * 4;
      if (dz) {
        const thr = 0.5 + dAmount * BAYER8[((Math.floor(y / dScale) & 7) << 3) | (Math.floor(x / dScale) & 7)];
        for (let c = 0; c < 3; c++) {
          const v = clamp01(rgb[o3 + c] / 255) * (levels - 1);
          out[o4 + c] = (Math.floor(v + thr) / (levels - 1)) * 255;
        }
      } else {
        const n = hash01(x, y, 1) - 0.5;
        out[o4] = rgb[o3] + n;
        out[o4 + 1] = rgb[o3 + 1] + n;
        out[o4 + 2] = rgb[o3 + 2] + n;
      }
      out[o4 + 3] = 255;
    }
  }
  return out;
}

/** String → RGBA. */
export function generate(seed: string, width = 512, height = 512, options: RenderOptions = {}): Uint8ClampedArray {
  return renderPattern(createPattern(seed), width, height, options);
}
