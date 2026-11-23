/** Flow field: evenly spaced streamlines (Jobard–Lefer) following a noise angle field, as tapered strokes. */
import { PI, TAU, clamp, cos, fbm2, hexRgb, hypot2, lerp, newImage, pow, put, round, sin, stream, type V3 } from "./base.js";
import { Mask, capsule, fillPolygon, paint } from "./raster.js";
import { type Generator, num, opt, str } from "./spec.js";

/** HSL (h in degrees, s and l in 0…1) to sRGB 0…255. */
export function hsl(h: number, s: number, l: number): V3 {
  h = (((h % 360) + 360) % 360) / 360;
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  const f = (t: number): number => {
    if (t < 0) t += 1;
    if (t > 1) t -= 1;
    if (t < 1 / 6) return p + (q - p) * 6 * t;
    if (t < 1 / 2) return q;
    if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
    return p;
  };
  return [f(h + 1 / 3) * 255, f(h) * 255, f(h - 1 / 3) * 255];
}

const BG: Record<string, string> = { ink: "#f3efe6", neon: "#06070e", gold: "#0d1a35", sunset: "#1c0e2a" };

export const flow: Generator = {
  id: "flow-field",
  title: "Flow field",
  description: "Thousands of evenly spaced, tapered streamlines that follow a noise field, like plotter art.",
  aspect: 1.5,
  params: [
    { kind: "choice", key: "style", label: "Style", default: "ink", options: [opt("ink", "Ink"), opt("neon", "Neon"), opt("gold", "Gold"), opt("sunset", "Sunset")] },
    { kind: "number", key: "spacing", label: "Spacing", min: 0.005, max: 0.022, step: 0.0005, default: 0.009 },
    { kind: "number", key: "swirl", label: "Swirl", min: 0.5, max: 4, step: 0.05, default: 1.6, unit: "x" },
    { kind: "number", key: "weight", label: "Weight", min: 0.4, max: 2.6, step: 0.05, default: 1.2, unit: "x" },
    { kind: "number", key: "length", label: "Length", min: 0.1, max: 1, step: 0.02, default: 0.7, unit: "%" },
  ],
  previews: [
    { title: "Style", sub: "Ink on paper, glowing neon, gold on navy, sunset.", key: "style", values: ["ink", "neon", "gold", "sunset"], labels: ["Ink", "Neon", "Gold", "Sunset"] },
    { title: "Swirl", sub: "From long calm currents to small eddies.", key: "swirl", values: [0.7, 1.6, 2.8, 4], labels: ["Calm", "Flow", "Eddy", "Turbulent"] },
  ],
  render(seed, W, H, p) {
    const R = stream(seed, "flow");
    const S = Math.floor(R() * 1e6);
    const ox = R() * 100;
    const oy = R() * 100;
    const tilt = R() * TAU;
    const style = str(p, "style");
    const bg = hexRgb(BG[style]);
    const img = new Float64Array(W * H * 3);
    for (let i = 0; i < W * H; i++) {
      img[i * 3] = bg[0];
      img[i * 3 + 1] = bg[1];
      img[i * 3 + 2] = bg[2];
    }
    const sc = num(p, "swirl") / H;
    const ang = (x: number, y: number): number => (fbm2(x * sc + ox, y * sc + oy, S, 4) - 0.5) * TAU * 2.6 + tilt;
    const dsep = Math.max(3, W * num(p, "spacing"));
    const cell = dsep / Math.SQRT2;
    const gw = Math.ceil(W / cell) + 1;
    const gh = Math.ceil(H / cell) + 1;
    const grid: number[][] = [];
    for (let i = 0; i < gw * gh; i++) grid.push([]);
    const step = dsep * 0.3;
    const maxSteps = 30 + round(num(p, "length") * 320);
    const free = (x: number, y: number, dt: number): boolean => {
      const cx = Math.floor(x / cell);
      const cy = Math.floor(y / cell);
      const r = Math.ceil(dt / cell);
      const t2 = dt * dt;
      for (let j = cy - r; j <= cy + r; j++) {
        if (j < 0 || j >= gh) continue;
        for (let i = cx - r; i <= cx + r; i++) {
          if (i < 0 || i >= gw) continue;
          const c = grid[j * gw + i];
          for (let k = 0; k < c.length; k += 2) {
            const dx = c[k] - x;
            const dy = c[k + 1] - y;
            if (dx * dx + dy * dy < t2) return false;
          }
        }
      }
      return true;
    };
    const trace = (sx: number, sy: number, back: boolean): number[] => {
      const out: number[] = [];
      let x = sx;
      let y = sy;
      const flip = back ? PI : 0;
      for (let n = 0; n < maxSteps; n++) {
        const a1 = ang(x, y) + flip;
        const mx = x + cos(a1) * step * 0.5;
        const my = y + sin(a1) * step * 0.5;
        const a2 = ang(mx, my) + flip;
        x += cos(a2) * step;
        y += sin(a2) * step;
        if (x < 0 || y < 0 || x >= W || y >= H || !free(x, y, dsep * 0.62)) break;
        out.push(x, y);
      }
      return out;
    };
    const lines: number[][] = [];
    const tries = round(((W * H) / (dsep * dsep)) * 1.4);
    for (let t = 0; t < tries; t++) {
      const x = R() * W;
      const y = R() * H;
      if (!free(x, y, dsep)) continue;
      const f = trace(x, y, false);
      const b = trace(x, y, true);
      const pts: number[] = [];
      for (let i = b.length - 2; i >= 0; i -= 2) pts.push(b[i], b[i + 1]);
      pts.push(x, y);
      for (const q of f) pts.push(q);
      if (pts.length < 12) continue;
      for (let i = 0; i < pts.length; i += 2) grid[Math.floor(pts[i + 1] / cell) * gw + Math.floor(pts[i] / cell)].push(pts[i], pts[i + 1]);
      lines.push(pts);
    }
    const wmax = dsep * 0.34 * num(p, "weight");
    const m = new Mask(W, H);
    const xs: number[] = [];
    const ys: number[] = [];
    for (const pts of lines) {
      const n = pts.length / 2;
      xs.length = 0;
      ys.length = 0;
      const rx: number[] = [];
      const ry: number[] = [];
      for (let i = 0; i < n; i++) {
        const i0 = Math.max(0, i - 1);
        const i1 = Math.min(n - 1, i + 1);
        let tx = pts[i1 * 2] - pts[i0 * 2];
        let ty = pts[i1 * 2 + 1] - pts[i0 * 2 + 1];
        const l = hypot2(tx, ty) || 1;
        tx /= l;
        ty /= l;
        const t = n > 1 ? i / (n - 1) : 0.5;
        const w = wmax * Math.max(0.12, pow(sin(PI * t), 0.55));
        xs.push(pts[i * 2] - ty * w);
        ys.push(pts[i * 2 + 1] + tx * w);
        rx.push(pts[i * 2] + ty * w);
        ry.push(pts[i * 2 + 1] - tx * w);
      }
      for (let i = n - 1; i >= 0; i--) {
        xs.push(rx[i]);
        ys.push(ry[i]);
      }
      const mid = n >> 1;
      const a = ang(pts[mid * 2], pts[mid * 2 + 1]);
      const my = pts[mid * 2 + 1] / H;
      const mx = pts[mid * 2] / W;
      let col: V3;
      let alpha = 1;
      if (style === "ink") {
        col = [21, 21, 26];
        alpha = 0.78 + 0.2 * R();
      } else if (style === "neon") {
        const h = 190 + 110 * sin(a * 0.9 + mx * 3);
        col = hsl(h, 1, 0.64);
        for (let i = 0; i + 1 < n; i++) capsule(m, pts[i * 2], pts[i * 2 + 1], pts[i * 2 + 2], pts[i * 2 + 3], wmax * 3.4);
        const g = hsl(h, 1, 0.6);
        paint(m, img, g[0], g[1], g[2], 0.16);
      } else if (style === "gold") {
        const h = 36 + 14 * sin(a) + 8 * R();
        const s = (62 + 20 * R()) / 100;
        col = hsl(h, s, (52 + 16 * R()) / 100);
      } else {
        const h = lerp(345, 50, clamp(my + 0.18 * sin(a), 0, 1));
        col = hsl(h, 0.92, (58 + 8 * R()) / 100);
      }
      fillPolygon(m, xs, ys, xs.length);
      paint(m, img, col[0], col[1], col[2], alpha);
    }
    const d = newImage(W, H);
    for (let i = 0; i < W * H; i++) put(d, i, img[i * 3], img[i * 3 + 1], img[i * 3 + 2]);
    return d;
  },
};
