/** Ebru and suminagashi: mathematical marbling (area-preserving maps applied to ink boundaries). */
import { LN2_, TAU, clamp, exp, fbmS, hash2, hexRgb, hypot2, newImage, put, round, stream, blur, pow, type V3 } from "./base.js";
import { Mask, capsule, fillPolygon, paint } from "./raster.js";
import { type Generator, type ParamValues, num, opt, str } from "./spec.js";
import { cos, sin } from "../dmath.js";

interface Blob {
  x: number[];
  y: number[];
  c: V3;
  m: number;
}
const F = new Float64Array(2);

type MapFn = (x: number, y: number, o: Float64Array) => void;

export class Marble {
  P: Blob[] = [];
  constructor(readonly seg: number, readonly cap = 7000) {}
  apply(fn: MapFn): void {
    const maxL = this.seg * 1.6;
    for (const p of this.P) {
      const X = p.x;
      const Y = p.y;
      const n = X.length;
      const mx = new Float64Array(n);
      const my = new Float64Array(n);
      for (let i = 0; i < n; i++) {
        fn(X[i], Y[i], F);
        mx[i] = F[0];
        my[i] = F[1];
      }
      const nx: number[] = [];
      const ny: number[] = [];
      let budget = Math.max(0, this.cap - n);
      for (let i = 0; i < n; i++) {
        nx.push(mx[i]);
        ny.push(my[i]);
        const j = i + 1 < n ? i + 1 : 0;
        const L = hypot2(mx[j] - mx[i], my[j] - my[i]);
        if (L > maxL && budget > 0) {
          const m = Math.min(12, Math.ceil(L / maxL), budget + 1);
          for (let s = 1; s < m; s++) {
            const t = s / m;
            fn(X[i] + (X[j] - X[i]) * t, Y[i] + (Y[j] - Y[i]) * t, F);
            nx.push(F[0]);
            ny.push(F[1]);
          }
          budget -= m - 1;
        }
      }
      p.x = nx;
      p.y = ny;
    }
  }
  drop(cx: number, cy: number, r: number, c: V3, meta: number): void {
    const r2 = r * r;
    this.apply((x, y, o) => {
      const dx = x - cx;
      const dy = y - cy;
      const d2 = dx * dx + dy * dy + 1e-14;
      const k = Math.sqrt(1 + r2 / d2);
      o[0] = cx + dx * k;
      o[1] = cy + dy * k;
    });
    const n = Math.max(36, Math.min(600, Math.ceil((TAU * r) / this.seg)));
    const xs: number[] = [];
    const ys: number[] = [];
    for (let i = 0; i < n; i++) {
      const a = (i / n) * TAU;
      xs.push(cx + r * cos(a));
      ys.push(cy + r * sin(a));
    }
    this.P.push({ x: xs, y: ys, c, m: meta });
  }
  tine(ax: number, ay: number, mx: number, my: number, z: number, c: number): void {
    const l = hypot2(mx, my);
    mx /= l;
    my /= l;
    const nx = -my;
    const ny = mx;
    const k = LN2_ / c;
    this.apply((x, y, o) => {
      const d = Math.abs((x - ax) * nx + (y - ay) * ny);
      const f = z * exp(-d * k);
      o[0] = x + f * mx;
      o[1] = y + f * my;
    });
  }
  comb(ax: number, ay: number, mx: number, my: number, z: number, c: number, sp: number, alt: boolean): void {
    const l = hypot2(mx, my);
    mx /= l;
    my /= l;
    const nx = -my;
    const ny = mx;
    const k = LN2_ / c;
    this.apply((x, y, o) => {
      const t = (x - ax) * nx + (y - ay) * ny;
      const k0 = round(t / sp);
      let f = 0;
      for (let q = k0 - 1; q <= k0 + 1; q++) {
        const d = Math.abs(t - q * sp);
        f += (alt && (q & 1) !== 0 ? -1 : 1) * z * exp(-d * k);
      }
      o[0] = x + f * mx;
      o[1] = y + f * my;
    });
  }
  swirl(cx: number, cy: number, a: number, R: number): void {
    this.apply((x, y, o) => {
      const dx = x - cx;
      const dy = y - cy;
      const r = hypot2(dx, dy);
      const th = a * exp(-r / R);
      const cs = cos(th);
      const sn = sin(th);
      o[0] = cx + dx * cs - dy * sn;
      o[1] = cy + dx * sn + dy * cs;
    });
  }
  breath(amp: number, freq: number, s: number, oct: number): void {
    const e = 1e-3;
    const psi = (x: number, y: number): number => fbmS(x * freq, y * freq, s, oct);
    this.apply((x, y, o) => {
      const vx = (psi(x, y + e) - psi(x, y - e)) / (2 * e);
      const vy = -(psi(x + e, y) - psi(x - e, y)) / (2 * e);
      o[0] = x + vx * amp;
      o[1] = y + vy * amp;
    });
  }
}

const mix3 = (a: V3, b: V3, t: number): V3 => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];

/** Paper: soft clouding, grain and a few fibres (RGB 0…255, multiplied over the print). */
function paperTexture(W: number, H: number, salt: string, strength: number): Float64Array {
  const P = new Float64Array(W * H * 3);
  const s = Math.floor(stream(salt, "paper")() * 9973);
  const sc = 600 / Math.max(W, H);
  for (let y = 0; y < H; y++)
    for (let x = 0; x < W; x++) {
      const cl = fbmS(x * sc * 0.012, y * sc * 0.012, s, 4);
      const g = hash2(x, y, s + 7) - 0.5;
      const v = 255 * (1 - strength * (0.05 * cl + 0.035 * g + 0.02));
      const i = (y * W + x) * 3;
      P[i] = v;
      P[i + 1] = v * 0.995;
      P[i + 2] = v * 0.985;
    }
  const r = stream(salt, "fibres");
  const n = round(((W * H) / 2600) * strength);
  const m = new Mask(W, H);
  for (let i = 0; i < n; i++) {
    const x = r() * W;
    const y = r() * H;
    const a = r() * TAU;
    const l = ((6 + r() * 26) * W) / 1000;
    const cv = (r() - 0.5) * l * 0.6;
    const dark = r() < 0.5;
    const lw = ((0.4 + r() * 0.7) * W) / 1000;
    const ca = cos(a);
    const sa = sin(a);
    const qx = x + (ca * l) / 2 - sa * cv;
    const qy = y + (sa * l) / 2 + ca * cv;
    const ex = x + ca * l;
    const ey = y + sa * l;
    let px = x;
    let py = y;
    for (let k = 1; k <= 6; k++) {
      const t = k / 6;
      const u = 1 - t;
      const nx = u * u * x + 2 * u * t * qx + t * t * ex;
      const ny = u * u * y + 2 * u * t * qy + t * t * ey;
      capsule(m, px, py, nx, ny, lw);
      px = nx;
      py = ny;
    }
    if (dark) paint(m, P, 120, 100, 70, 0.1);
    else paint(m, P, 255, 255, 255, 0.35);
  }
  return P;
}

function finish(img: Float64Array, paper: Float64Array, W: number, H: number): Uint8ClampedArray {
  const d = newImage(W, H);
  for (let i = 0; i < W * H; i++)
    put(d, i, (img[i * 3] * paper[i * 3]) / 255, (img[i * 3 + 1] * paper[i * 3 + 1]) / 255, (img[i * 3 + 2] * paper[i * 3 + 2]) / 255);
  return d;
}

function drawBlobs(mk: Marble, W: number, H: number, img: Float64Array, outline: (b: Blob) => [V3, number, number] | null): void {
  const fill = new Mask(W, H);
  const line = new Mask(W, H);
  const xs: number[] = [];
  const ys: number[] = [];
  for (const p of mk.P) {
    const n = p.x.length;
    xs.length = 0;
    ys.length = 0;
    for (let i = 0; i < n; i++) {
      xs.push(p.x[i] * W);
      ys.push(p.y[i] * W);
    }
    fillPolygon(fill, xs, ys, n);
    paint(fill, img, p.c[0], p.c[1], p.c[2], 1);
    const o = outline(p);
    if (o) {
      for (let i = 0; i < n; i++) {
        const j = i + 1 < n ? i + 1 : 0;
        capsule(line, xs[i], ys[i], xs[j], ys[j], o[2]);
      }
      paint(line, img, o[0][0], o[0][1], o[0][2], o[1]);
    }
  }
}

// ---------------- ebru ----------------
const EBRU_PALS: Record<string, { bg: string; inks: string[] }> = {
  klasik: { bg: "#efe6d0", inks: ["#1f2c5c", "#c8963e", "#8c2f2b", "#5f7a5a", "#efe6d0"] },
  deniz: { bg: "#e9e3d3", inks: ["#123a63", "#2f8f9d", "#d9c9a3", "#0b1f3a", "#f1ead8"] },
  gul: { bg: "#f2e7dc", inks: ["#7a1f33", "#c9566b", "#d9b46a", "#8a9a78", "#f4ece2"] },
  toprak: { bg: "#e8dcc2", inks: ["#5b3a24", "#a0522d", "#c99a3b", "#6b6b3a", "#efe4cc"] },
  lale: { bg: "#f0e8d4", inks: ["#b3261e", "#e0a526", "#2e6b3f", "#2a4a9b", "#f2ead6"] },
};
const EBRU_PAL_IDS = ["klasik", "deniz", "gul", "toprak", "lale"];

function ebruPlan(seed: string, p: ParamValues, h: number, mk: Marble): V3 {
  const R = stream(seed, "ebru");
  const palId = str(p, "palette") === "auto" ? EBRU_PAL_IDS[Math.floor(stream(seed, "ebru-pal")() * 5)] : str(p, "palette");
  const pal = EBRU_PALS[palId];
  const inks = pal.inks.map(hexRgb);
  const style = str(p, "style");
  const S = num(p, "size");
  const D = num(p, "density");
  const C = num(p, "comb");
  const battal = (scale: number, count: number): void => {
    const rounds = inks.length;
    for (let k = 0; k < rounds; k++) {
      const n = round(((count * (0.6 + 0.8 * R())) / rounds) * (k === rounds - 1 ? 1.4 : 1));
      for (let i = 0; i < n; i++) {
        const x = -0.05 + R() * 1.1;
        const y = -0.05 + R() * (h + 0.1);
        const q = R();
        mk.drop(x, y, (0.016 + q * q * 0.055) * scale, inks[k], 0);
      }
    }
  };
  if (style === "hatip") {
    battal(S * 0.8, round(200 * D));
    const sp = 0.16 / Math.sqrt(D);
    const rings = [inks[2], inks[4], inks[0], inks[1]].slice(0, 3 + (R() < 0.5 ? 1 : 0));
    const cols = Math.ceil(1 / sp) + 1;
    const rows = Math.ceil(h / sp) + 1;
    const base = (0.034 + 0.01 * R()) * S;
    for (let j = 0; j < rows; j++)
      for (let i = 0; i < cols; i++) {
        const x = (i + 0.5 + (j & 1) * 0.5) * sp - sp * 0.25;
        const y = (j + 0.5) * sp;
        for (let q = 0; q < rings.length; q++) mk.drop(x, y, base * (1 - q * 0.22), rings[q], 0);
      }
    for (let i = 0; i < cols * 2; i++) mk.tine((i * 0.5 + 0.5) * sp - sp * 0.25, 0, 0, 1, 0.06, 0.011);
    return hexRgb(pal.bg);
  }
  battal(S, round(320 * D));
  if (style !== "battal") {
    const sp = 0.075 / C;
    mk.comb(R() * sp, 0, 0, 1, 0.07, 0.012, sp, true);
    const sp2 = 0.09 / C;
    mk.comb(0, R() * sp2, 1, 0, 0.06, 0.012, sp2, true);
  }
  if (style === "tarakli" || style === "bulbul-yuvasi") {
    const sp = 0.022 / C;
    const ph = R() * sp;
    const dir = R() < 0.5 ? 1 : -1;
    mk.comb(ph, 0, 0, dir, 0.05, 0.006, sp, false);
  }
  if (style === "bulbul-yuvasi") {
    const sp = 0.11 / Math.sqrt(C);
    const dir = R() < 0.5 ? 1 : -1;
    const ny = Math.ceil(h / sp - 0.5);
    const nx = Math.ceil(1 / sp - 0.5);
    for (let j = 0; j < ny; j++)
      for (let i = 0; i < nx; i++) {
        const xx = sp / 2 + i * sp + (R() - 0.5) * sp * 0.15;
        const yy = sp / 2 + j * sp + (R() - 0.5) * sp * 0.15;
        mk.swirl(xx, yy, dir * (9 + R() * 4), sp * 0.17);
      }
  }
  return hexRgb(pal.bg);
}

function segFor(W: number): number {
  return W >= 900 ? 0.0028 : W >= 500 ? 0.004 : 0.006;
}

export const ebru: Generator = {
  id: "ebru",
  title: "Ebru",
  description: "Turkish paper marbling: drops of paint sprinkled on size, then combed, raked and swirled.",
  aspect: 1.5,
  params: [
    {
      kind: "choice",
      key: "style",
      label: "Style",
      default: "tarakli",
      options: [opt("battal", "Battal"), opt("gel-git", "Gel-git"), opt("tarakli", "Taraklı"), opt("bulbul-yuvasi", "Bülbül yuvası"), opt("hatip", "Hatip")],
    },
    {
      kind: "choice",
      key: "palette",
      label: "Palette",
      default: "auto",
      options: [opt("auto", "Auto"), opt("klasik", "Klasik"), opt("deniz", "Deniz"), opt("gul", "Gül"), opt("toprak", "Toprak"), opt("lale", "Lale")],
    },
    { kind: "number", key: "density", label: "Density", min: 0.3, max: 1.6, step: 0.05, default: 1, unit: "x" },
    { kind: "number", key: "size", label: "Drop size", min: 0.5, max: 1.8, step: 0.05, default: 1, unit: "x" },
    { kind: "number", key: "comb", label: "Comb", min: 0.5, max: 2, step: 0.05, default: 1, unit: "x" },
  ],
  previews: [
    { title: "Style", sub: "Five classic patterns from the same drops.", key: "style", values: ["battal", "gel-git", "tarakli", "bulbul-yuvasi", "hatip"], labels: ["Battal", "Gel-git", "Taraklı", "Bülbül yuvası", "Hatip"] },
    { title: "Palette", sub: "Traditional pigment sets.", key: "palette", values: ["klasik", "deniz", "gul", "toprak", "lale"], labels: ["Klasik", "Deniz", "Gül", "Toprak", "Lale"] },
  ],
  render(seed, W, H, p) {
    const mk = new Marble(segFor(W));
    const bg = ebruPlan(seed, p, H / W, mk);
    const img = new Float64Array(W * H * 3);
    for (let i = 0; i < W * H; i++) {
      img[i * 3] = bg[0];
      img[i * 3 + 1] = bg[1];
      img[i * 3 + 2] = bg[2];
    }
    const lw = Math.max(0.6, W / 1100);
    drawBlobs(mk, W, H, img, (b) => [mix3(b.c, [0, 0, 0], 0.35), 0.45, lw]);
    return finish(img, paperTexture(W, H, "paper", 1), W, H);
  },
};

// ---------------- suminagashi ----------------
const SUMI_PAPER: V3 = hexRgb("#f5f1e7");
const SUMI_INKS: Record<string, string[]> = {
  sumi: ["#121212"],
  ai: ["#1d3566"],
  "sumi-ai": ["#121212", "#1d3566"],
  beni: ["#121212", "#121212", "#121212", "#b3322a"],
};

function sumiPlan(seed: string, p: ParamValues, h: number, mk: Marble): void {
  const R = stream(seed, "sumi");
  const inks = SUMI_INKS[str(p, "ink")].map(hexRgb);
  const nC = num(p, "centers") >= 1 ? round(num(p, "centers")) : 1 + Math.floor(R() * 3);
  const rings = num(p, "rings");
  const C: { x: number; y: number; n: number; w: number; k: number; r: number }[] = [];
  for (let i = 0; i < nC; i++) {
    const x = 0.12 + R() * 0.76;
    const y = h * (0.12 + R() * 0.76);
    const n = Math.max(4, round((rings * (0.65 + 0.7 * R())) / Math.sqrt(nC * 0.8 + 0.2)));
    C.push({ x, y, n, w: 0.7 + 0.6 * R(), k: 0, r: 0 });
  }
  let total = 0;
  for (const c of C) total += c.n;
  const rBase = Math.sqrt((0.8 * h) / (2.53 * Math.PI * total));
  for (const c of C) c.r = rBase * c.w;
  for (let t = 0; t < total; t++) {
    const live = C.filter((c) => c.k < c.n);
    if (!live.length) break;
    const c = live[Math.floor(R() * live.length)];
    for (let pass = 0; pass < 2; pass++) {
      const x = c.x + (R() - 0.5) * 0.003;
      const y = c.y + (R() - 0.5) * 0.003;
      if (pass === 0) {
        const base = inks[Math.floor(R() * inks.length)];
        const dens = 0.45 + 0.55 * pow(R(), 0.6);
        mk.drop(x, y, c.r * (0.75 + 0.55 * R()), mix3(SUMI_PAPER, base, dens), 1);
      } else mk.drop(x, y, c.r * (0.85 + 0.7 * R()), SUMI_PAPER, 0);
    }
    c.k++;
  }
  const b = num(p, "breath");
  if (b > 0) {
    const steps = 4 + round(6 * b);
    const freq = 0.7 + 0.9 * R();
    const s = Math.floor(R() * 1e6);
    for (let i = 0; i < steps; i++) mk.breath(0.022 * b, freq, s + i * 3, 2);
    const fans = 1 + Math.floor(R() * 2);
    for (let i = 0; i < fans; i++) {
      const ax = R();
      const ay = R() * h;
      const a = R() * TAU;
      const z = (0.06 + 0.08 * R()) * b;
      mk.tine(ax, ay, cos(a), sin(a), z, 0.18);
    }
    if (R() < 0.6) {
      const sx = 0.2 + R() * 0.6;
      const sy = h * (0.2 + R() * 0.6);
      const a = (R() < 0.5 ? -1 : 1) * (1.2 + 2 * R()) * b;
      mk.swirl(sx, sy, a, 0.12);
    }
  }
}

export const suminagashi: Generator = {
  id: "suminagashi",
  title: "Suminagashi",
  description: "Japanese floating ink: alternating drops of ink and clear water make concentric rings that a breath distorts.",
  aspect: 1.5,
  params: [
    { kind: "choice", key: "ink", label: "Ink", default: "sumi", options: [opt("sumi", "Sumi"), opt("ai", "Ai"), opt("sumi-ai", "Sumi + Ai"), opt("beni", "Beni")] },
    { kind: "number", key: "rings", label: "Rings", min: 8, max: 70, step: 1, default: 34 },
    { kind: "number", key: "centers", label: "Centres (0 = auto)", min: 0, max: 6, step: 1, default: 0 },
    { kind: "number", key: "breath", label: "Breath", min: 0, max: 1, step: 0.01, default: 0.45, unit: "%" },
  ],
  previews: [
    { title: "Ink", sub: "Black sumi, indigo ai, both, and a touch of beni red.", key: "ink", values: ["sumi", "ai", "sumi-ai", "beni"], labels: ["Sumi", "Ai", "Sumi + Ai", "Beni"] },
    { title: "Breath", sub: "From still water to a gust across the tray.", key: "breath", values: [0, 0.35, 0.65, 1], labels: ["Still", "Breath", "Breeze", "Wind"] },
  ],
  render(seed, W, H, p) {
    const mk = new Marble(segFor(W) * 0.93);
    sumiPlan(seed, p, H / W, mk);
    let img = new Float64Array(W * H * 3);
    for (let i = 0; i < W * H; i++) {
      img[i * 3] = SUMI_PAPER[0];
      img[i * 3 + 1] = SUMI_PAPER[1];
      img[i * 3 + 2] = SUMI_PAPER[2];
    }
    const lw = Math.max(0.5, W / 1600);
    drawBlobs(mk, W, H, img, (b) => (b.m ? [b.c, 0.5, lw] : null));
    // ink lifted by the paper: slightly feathered
    const br = round(W / 1500);
    if (br >= 1) {
      const ch = [new Float64Array(W * H), new Float64Array(W * H), new Float64Array(W * H)];
      for (let i = 0; i < W * H; i++) for (let c = 0; c < 3; c++) ch[c][i] = img[i * 3 + c];
      const out = ch.map((a) => blur(a, W, H, br));
      img = new Float64Array(W * H * 3);
      for (let i = 0; i < W * H; i++) for (let c = 0; c < 3; c++) img[i * 3 + c] = out[c][i];
    }
    // uneven absorption: screen a soft mottle over it
    const sc = 700 / Math.max(W, H);
    for (let y = 0; y < H; y++)
      for (let x = 0; x < W; x++) {
        const f = fbmS(x * sc * 0.02, y * sc * 0.02, 404, 4);
        const g = hash2(x, y, 77);
        const mv = clamp(Math.max(0, f - 0.42) * 190 + g * 14, 0, 255) / 255;
        const i = (y * W + x) * 3;
        for (let c = 0; c < 3; c++) {
          const a = img[i + c];
          const s = 255 - ((255 - a) * (255 - mv * 255)) / 255;
          img[i + c] = a + (s - a) * 0.55;
        }
      }
    return finish(img, paperTexture(W, H, "washi", 1), W, H);
  },
};
