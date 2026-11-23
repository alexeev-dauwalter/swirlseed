/**
 * A small deterministic rasteriser (the browser canvas differs between engines, so the generators
 * never use it). Coverage is exact horizontally and sampled on 4 sub-rows per pixel vertically.
 * `crates/swirlseed/src/gen/raster.rs` mirrors this file.
 */
import { clamp } from "./core.js";

const SUB = 4;

/** A coverage mask (0…1 per pixel) with the bounding box of what was drawn since the last clear. */
export class Mask {
  readonly cov: Float64Array;
  x0 = 0;
  y0 = 0;
  x1 = -1;
  y1 = -1;
  constructor(readonly W: number, readonly H: number) {
    this.cov = new Float64Array(W * H);
  }
  clear(): void {
    for (let y = this.y0; y <= this.y1; y++) for (let x = this.x0; x <= this.x1; x++) this.cov[y * this.W + x] = 0;
    this.x0 = 0;
    this.y0 = 0;
    this.x1 = -1;
    this.y1 = -1;
  }
  grow(x0: number, y0: number, x1: number, y1: number): void {
    if (this.x1 < this.x0) {
      this.x0 = x0;
      this.y0 = y0;
      this.x1 = x1;
      this.y1 = y1;
      return;
    }
    this.x0 = Math.min(this.x0, x0);
    this.y0 = Math.min(this.y0, y0);
    this.x1 = Math.max(this.x1, x1);
    this.y1 = Math.max(this.y1, y1);
  }
}

/** Fill a closed polygon (non-zero winding) into the mask, adding coverage. */
export function fillPolygon(m: Mask, xs: ArrayLike<number>, ys: ArrayLike<number>, n: number): void {
  const W = m.W;
  const H = m.H;
  let ymin = Infinity;
  let ymax = -Infinity;
  for (let i = 0; i < n; i++) {
    if (ys[i] < ymin) ymin = ys[i];
    if (ys[i] > ymax) ymax = ys[i];
  }
  const k0 = Math.max(0, Math.ceil(ymin * SUB - 0.5));
  const k1 = Math.min(H * SUB - 1, Math.ceil(ymax * SUB - 0.5) - 1);
  if (k1 < k0) return;
  const rows: number[][] = [];
  for (let k = k0; k <= k1; k++) rows.push([]);
  for (let i = 0; i < n; i++) {
    const j = i + 1 < n ? i + 1 : 0;
    const xa = xs[i];
    const ya = ys[i];
    const xb = xs[j];
    const yb = ys[j];
    if (ya === yb) continue;
    const dir = yb > ya ? 1 : -1;
    const lo = Math.min(ya, yb);
    const hi = Math.max(ya, yb);
    const ka = Math.max(k0, Math.ceil(lo * SUB - 0.5));
    const kb = Math.min(k1, Math.ceil(hi * SUB - 0.5) - 1);
    const sl = (xb - xa) / (yb - ya);
    for (let k = ka; k <= kb; k++) {
      const sy = (k + 0.5) / SUB;
      rows[k - k0].push(xa + (sy - ya) * sl, dir);
    }
  }
  let bx0 = W;
  let bx1 = -1;
  const idx: number[] = [];
  for (let k = k0; k <= k1; k++) {
    const r = rows[k - k0];
    const c = r.length / 2;
    if (c < 2) continue;
    idx.length = 0;
    for (let q = 0; q < c; q++) idx.push(q);
    idx.sort((p, q) => r[p * 2] - r[q * 2] || r[p * 2 + 1] - r[q * 2 + 1] || p - q);
    const py = Math.floor(k / SUB);
    const row = py * W;
    let wind = 0;
    for (let q = 0; q < c - 1; q++) {
      wind += r[idx[q] * 2 + 1];
      if (wind === 0) continue;
      const xa = clamp(r[idx[q] * 2], 0, W);
      const xb = clamp(r[idx[q + 1] * 2], 0, W);
      if (xb <= xa) continue;
      const ia = Math.floor(xa);
      const ib = Math.min(W - 1, Math.ceil(xb) - 1);
      for (let x = ia; x <= ib; x++) {
        const ov = Math.min(xb, x + 1) - Math.max(xa, x);
        if (ov > 0) m.cov[row + x] += ov / SUB;
      }
      if (ia < bx0) bx0 = ia;
      if (ib > bx1) bx1 = ib;
    }
  }
  if (bx1 >= bx0) m.grow(bx0, Math.floor(k0 / SUB), bx1, Math.floor(k1 / SUB));
}

/** Round-capped segment of width w (antialiased by distance); coverage is max-combined. */
export function capsule(m: Mask, ax: number, ay: number, bx: number, by: number, w: number): void {
  const r = w / 2;
  const x0 = Math.max(0, Math.floor(Math.min(ax, bx) - r - 1));
  const x1 = Math.min(m.W - 1, Math.ceil(Math.max(ax, bx) + r + 1));
  const y0 = Math.max(0, Math.floor(Math.min(ay, by) - r - 1));
  const y1 = Math.min(m.H - 1, Math.ceil(Math.max(ay, by) + r + 1));
  if (x1 < x0 || y1 < y0) return;
  const dx = bx - ax;
  const dy = by - ay;
  const ll = dx * dx + dy * dy;
  for (let y = y0; y <= y1; y++)
    for (let x = x0; x <= x1; x++) {
      const px = x + 0.5 - ax;
      const py = y + 0.5 - ay;
      const t = ll > 0 ? clamp((px * dx + py * dy) / ll, 0, 1) : 0;
      const ex = px - dx * t;
      const ey = py - dy * t;
      const c = clamp(r - Math.sqrt(ex * ex + ey * ey) + 0.5, 0, 1) * (w < 1 ? w : 1);
      const i = y * m.W + x;
      if (c > m.cov[i]) m.cov[i] = c;
    }
  m.grow(x0, y0, x1, y1);
}

/** Filled disc; coverage is max-combined. */
export function disc(m: Mask, cx: number, cy: number, r: number): void {
  const x0 = Math.max(0, Math.floor(cx - r - 1));
  const x1 = Math.min(m.W - 1, Math.ceil(cx + r + 1));
  const y0 = Math.max(0, Math.floor(cy - r - 1));
  const y1 = Math.min(m.H - 1, Math.ceil(cy + r + 1));
  if (x1 < x0 || y1 < y0) return;
  for (let y = y0; y <= y1; y++)
    for (let x = x0; x <= x1; x++) {
      const ex = x + 0.5 - cx;
      const ey = y + 0.5 - cy;
      const c = clamp(r - Math.sqrt(ex * ex + ey * ey) + 0.5, 0, 1);
      const i = y * m.W + x;
      if (c > m.cov[i]) m.cov[i] = c;
    }
  m.grow(x0, y0, x1, y1);
}

/** Paint a solid colour through the mask into an RGB float buffer: dst = dst·(1 − a·m) + col·a·m. Clears the mask. */
export function paint(m: Mask, img: Float64Array, r: number, g: number, b: number, alpha: number): void {
  for (let y = m.y0; y <= m.y1; y++)
    for (let x = m.x0; x <= m.x1; x++) {
      const i = y * m.W + x;
      let c = m.cov[i];
      if (c <= 0) continue;
      if (c > 1) c = 1;
      const a = c * alpha;
      img[i * 3] += (r - img[i * 3]) * a;
      img[i * 3 + 1] += (g - img[i * 3 + 1]) * a;
      img[i * 3 + 2] += (b - img[i * 3 + 2]) * a;
      m.cov[i] = 0;
    }
  m.x0 = 0;
  m.y0 = 0;
  m.x1 = -1;
  m.y1 = -1;
}
