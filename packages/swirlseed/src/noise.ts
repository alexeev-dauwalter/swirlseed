import { TAU, cos, sin } from "./dmath.js";

/** Classic 2D gradient noise (16 gradient directions, quintic fade), range ≈ [-1, 1]. */
export class Noise {
  private readonly gx = new Float64Array(16);
  private readonly gy = new Float64Array(16);

  constructor(private readonly perm: Uint8Array) {
    for (let i = 0; i < 16; i++) {
      this.gx[i] = cos((i * TAU) / 16);
      this.gy[i] = sin((i * TAU) / 16);
    }
  }

  noise(x: number, y: number): number {
    const perm = this.perm;
    const xi = Math.floor(x);
    const yi = Math.floor(y);
    const xf = x - xi;
    const yf = y - yi;
    const X = xi & 255;
    const Y = yi & 255;
    const u = xf * xf * xf * (xf * (xf * 6 - 15) + 10);
    const v = yf * yf * yf * (yf * (yf * 6 - 15) + 10);
    const pa = perm[X];
    const pb = perm[X + 1];
    const g00 = perm[pa + Y] & 15;
    const g01 = perm[pa + Y + 1] & 15;
    const g10 = perm[pb + Y] & 15;
    const g11 = perm[pb + Y + 1] & 15;
    const n00 = this.gx[g00] * xf + this.gy[g00] * yf;
    const n10 = this.gx[g10] * (xf - 1) + this.gy[g10] * yf;
    const n01 = this.gx[g01] * xf + this.gy[g01] * (yf - 1);
    const n11 = this.gx[g11] * (xf - 1) + this.gy[g11] * (yf - 1);
    const a = n00 + u * (n10 - n00);
    const b = n01 + u * (n11 - n01);
    return (a + v * (b - a)) * 1.41;
  }

  /** Fractal Brownian motion with a rotation between octaves (hides the lattice). */
  fbm(x: number, y: number, oct: number): number {
    let sum = 0;
    let amp = 0.5;
    let norm = 0;
    for (let i = 0; i < oct; i++) {
      sum += amp * this.noise(x, y);
      norm += amp;
      const nx = (0.8 * x - 0.6 * y) * 2.03 + 17.3;
      y = (0.6 * x + 0.8 * y) * 2.03 + 9.1;
      x = nx;
      amp *= 0.5;
    }
    return sum / norm;
  }
}

/** Integer hash noise in [0, 1) for grain and dithering. x, y, k are small integers. */
export function hash01(x: number, y: number, k: number): number {
  let h = (Math.imul(x, 374761393) + Math.imul(y, 668265263) + Math.imul(k, 2246822519)) | 0;
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
}

/** Bayer 8×8 ordered-dither thresholds in (-0.5, 0.5). */
export const BAYER8: Float32Array = (() => {
  let m: number[][] = [[0]];
  while (m.length < 8) {
    const n = m.length;
    const next: number[][] = Array.from({ length: n * 2 }, () => new Array<number>(n * 2).fill(0));
    for (let y = 0; y < n; y++) {
      for (let x = 0; x < n; x++) {
        const v = m[y][x] * 4;
        next[y][x] = v;
        next[y][x + n] = v + 2;
        next[y + n][x] = v + 3;
        next[y + n][x + n] = v + 1;
      }
    }
    m = next;
  }
  const out = new Float32Array(64);
  for (let y = 0; y < 8; y++) for (let x = 0; x < 8; x++) out[y * 8 + x] = (m[y][x] + 0.5) / 64 - 0.5;
  return out;
})();

/** Separable box blur with edge clamping (running sum, O(N)). */
export function boxBlur(src: Float32Array, W: number, H: number, r: number): Float32Array {
  const tmp = new Float32Array(src.length);
  const out = new Float32Array(src.length);
  const d = 2 * r + 1;
  for (let y = 0; y < H; y++) {
    const row = y * W;
    let sum = 0;
    for (let k = -r; k <= r; k++) sum += src[row + Math.min(W - 1, Math.max(0, k))];
    for (let x = 0; x < W; x++) {
      tmp[row + x] = sum / d;
      sum += src[row + Math.min(W - 1, x + r + 1)] - src[row + Math.max(0, x - r)];
    }
  }
  for (let x = 0; x < W; x++) {
    let sum = 0;
    for (let k = -r; k <= r; k++) sum += tmp[Math.min(H - 1, Math.max(0, k)) * W + x];
    for (let y = 0; y < H; y++) {
      out[y * W + x] = sum / d;
      sum += tmp[Math.min(H - 1, y + r + 1) * W + x] - tmp[Math.max(0, y - r) * W + x];
    }
  }
  return out;
}
