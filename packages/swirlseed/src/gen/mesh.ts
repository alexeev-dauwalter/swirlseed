/** Mesh gradient: wide OKLab colour fields with folded silk-like sheets, a vortex and film grain. */
import { PI, TAU, atan2, cos, enc, exp, fbm2, hash2, hexRgb, hypot2, newImage, oklab2lin, oklchFit, pow, put, round, sin, srgbToLab, sstep, stream, vnoise, type V3 } from "./base.js";
import { type Generator, num, opt, str } from "./spec.js";

const PALS: Record<string, string[]> = {
  sky: ["#eef1f5", "#fbfcff", "#4f8fe6", "#f1ec97", "#a7c6f2", "#dcd6f2"],
  sunset: ["#cfe6fb", "#3b30e0", "#9a45d6", "#e27aa2", "#f5d2a6", "#fbf6c0", "#c75a3e"],
  lagoon: ["#1d8c90", "#0c2a44", "#d6f7a6", "#eef3b4", "#c99a1c", "#4cbfae"],
  neon: ["#26369a", "#a3288e", "#36c4e2", "#16246e", "#6b36aa", "#e04c9a"],
  mono: ["#f3f3f3", "#151515", "#8c8c8c", "#d6d6d6", "#454545"],
};
const HUE = [0, 40, 180, 210, -35, 90];
const LL = [0.95, 0.55, 0.75, 0.35, 0.85, 0.65];
const CC = [0.03, 0.18, 0.14, 0.12, 0.08, 0.16];

interface Fold {
  arc: boolean;
  cx: number;
  cy: number;
  r: number;
  c: number;
  th: number;
  a: number;
  f: number;
  ph: number;
  a2: number;
  f2: number;
  ph2: number;
  side: number;
  edge: V3;
  deep: V3;
  third: V3;
  depth: number;
  sh: number;
  off: number;
  op: number;
  fade: number;
  gf: number;
}

export const mesh: Generator = {
  id: "mesh-gradient",
  title: "Mesh gradient",
  description: "Soft gradients like blurred folded silk: OKLab colour fields, folds with crisp and misty edges, a vortex and film grain.",
  aspect: 1.5,
  params: [
    { kind: "choice", key: "palette", label: "Palette", default: "sunset", options: [opt("sky", "Sky"), opt("sunset", "Sunset"), opt("lagoon", "Lagoon"), opt("neon", "Neon"), opt("mono", "Mono"), opt("random", "Random")] },
    { kind: "number", key: "folds", label: "Folds", min: 0, max: 6, step: 1, default: 2 },
    { kind: "number", key: "soft", label: "Softness", min: 0, max: 1, step: 0.01, default: 0.65, unit: "%" },
    { kind: "number", key: "swirl", label: "Swirl", min: 0, max: 1, step: 0.01, default: 0.45, unit: "%" },
    { kind: "number", key: "flow", label: "Flow", min: 0, max: 1, step: 0.01, default: 0.5, unit: "%" },
    { kind: "number", key: "grain", label: "Grain", min: 0, max: 1, step: 0.01, default: 0.4, unit: "%" },
  ],
  previews: [
    { title: "Palette", sub: "Same seed, six colour sets.", key: "palette", values: ["sky", "sunset", "lagoon", "neon", "mono", "random"], labels: ["Sky", "Sunset", "Lagoon", "Neon", "Mono", "Random"] },
    { title: "Folds", sub: "From a plain gradient to layered silk.", key: "folds", values: [0, 2, 4, 6], labels: ["0", "2", "4", "6"] },
  ],
  render(seed, W, H, p) {
    const d = newImage(W, H);
    const R = stream(seed, "mesh");
    const S = Math.floor(R() * 1e6);
    const asp = W / H;
    let pal: V3[];
    if (str(p, "palette") === "random") {
      const h = R() * 360;
      pal = [];
      for (let i = 0; i < 6; i++) {
        const hh = h + HUE[i] + (R() - 0.5) * 20;
        pal.push(oklchFit(LL[i], CC[i], ((hh % 360) + 360) % 360));
      }
    } else pal = PALS[str(p, "palette")].map((c) => srgbToLab(hexRgb(c)));
    const np = pal.length;
    const pick = (): number => Math.floor(R() * np);
    const keys: number[] = [];
    for (let i = 0; i < np; i++) keys.push(R());
    const order: number[] = [];
    for (let i = 0; i < np; i++) order.push(i);
    order.sort((a, b) => keys[a] - keys[b] || a - b);
    const bp: { x: number; y: number; c: V3; s: number }[] = [];
    for (let i = 0; i < 6; i++) {
      const x = asp * (-0.15 + 1.3 * R());
      const y = -0.15 + 1.3 * R();
      bp.push({ x, y, c: pal[order[i % np]], s: 0.28 + 0.3 * R() });
    }
    const K = round(num(p, "folds"));
    const L: Fold[] = [];
    for (let k = 0; k < K; k++) {
      const arc = R() < 0.35;
      const t0 = (R() - 0.5) * 1.6;
      const th = t0 + (R() < 0.5 ? 0 : (PI / 2) * (R() - 0.5));
      const ei = pick();
      let di = pick();
      for (let t = 0; t < 4 && di === ei; t++) di = pick();
      const cx = asp * (-0.2 + 1.4 * R());
      const cy = -0.2 + 1.4 * R();
      const r = 0.35 + 0.6 * R();
      const c = 0.2 + 0.6 * R();
      const a = 0.08 + 0.22 * R();
      const f = 1.2 + 2.2 * R();
      const ph = R() * TAU;
      const a2 = 0.03 + 0.06 * R();
      const f2 = 3 + 3 * R();
      const ph2 = R() * TAU;
      const side = R() < 0.5 ? 1 : -1;
      const third = pal[pick()];
      const depth = 0.12 + 0.35 * R();
      const sh = 0.05 + 0.15 * R();
      const off = R() * 50;
      const op = 0.55 + 0.4 * R();
      const fade = 0.15 + 0.35 * R();
      const gf = 0.6 + 1.4 * R();
      L.push({ arc, cx, cy, r, c, th, a, f, ph, a2, f2, ph2, side, edge: pal[ei], deep: pal[di], third, depth, sh, off, op, fade, gf });
    }
    const vx = asp * (0.25 + 0.5 * R());
    const vy = 0.25 + 0.5 * R();
    const vsg = R() < 0.5 ? 1 : -1;
    const vs = vsg * (2.2 + 2 * R()) * num(p, "swirl");
    const vr = 0.18 + 0.25 * R();
    const soft = num(p, "soft");
    const gr = num(p, "grain") * 0.11;
    const flow = num(p, "flow");
    for (let y = 0; y < H; y++) {
      const v0 = y / H;
      for (let x = 0; x < W; x++) {
        let u = x / H;
        let v = v0;
        u += (fbm2(u * 1.1 + 3, v * 1.1, S, 3) - 0.5) * 0.35 * flow;
        v += (fbm2(u * 1.1, v * 1.1 + 7, S + 1, 3) - 0.5) * 0.35 * flow;
        {
          const dx = u - vx;
          const dy = v - vy;
          const ang = vs * exp(-(dx * dx + dy * dy) / (vr * vr));
          const ca = cos(ang);
          const sa = sin(ang);
          u = vx + dx * ca - dy * sa;
          v = vy + dx * sa + dy * ca;
        }
        let sw = 0;
        let c0 = 0;
        let c1 = 0;
        let c2 = 0;
        for (const q of bp) {
          const dx = u - q.x;
          const dy = v - q.y;
          const w = exp(-(dx * dx + dy * dy) / (q.s * q.s));
          sw += w;
          c0 += q.c[0] * w;
          c1 += q.c[1] * w;
          c2 += q.c[2] * w;
        }
        c0 /= sw;
        c1 /= sw;
        c2 /= sw;
        for (const l of L) {
          let sd: number;
          let along: number;
          if (l.arc) {
            const dx = u - l.cx;
            const dy = v - l.cy;
            sd = (l.r - hypot2(dx, dy)) * l.side;
            along = atan2(dy, dx) * l.r;
          } else {
            const ct = cos(l.th);
            const st = sin(l.th);
            const uu = u * ct + v * st;
            const vv = -u * st + v * ct;
            const y0 = l.c + l.a * sin(uu * l.f + l.ph) + l.a2 * sin(uu * l.f2 + l.ph2);
            const sl = l.a * l.f * cos(uu * l.f + l.ph) + l.a2 * l.f2 * cos(uu * l.f2 + l.ph2);
            sd = ((vv - y0) / Math.sqrt(1 + sl * sl)) * l.side;
            along = uu;
          }
          const sv = vnoise(along * 1.6 + l.off, 0.5, S + 7);
          const s = 0.004 + (0.03 + 0.38 * soft) * pow(sv, 1.2) + 0.03 * soft;
          const a = sstep(-s, s, sd);
          if (sd < 0) c0 *= 1 - exp(sd / l.sh) * 0.15;
          if (a > 0) {
            const t = sstep(0, l.depth, sd);
            const g = (0.5 + 0.5 * sin(along * l.gf + l.off)) * 0.7;
            const sdp = Math.max(0, sd);
            const hi = exp(-sdp / (s * 2 + 0.012)) * 0.07;
            const m0 = l.edge[0] + (l.third[0] - l.edge[0]) * g;
            const m1 = l.edge[1] + (l.third[1] - l.edge[1]) * g;
            const m2 = l.edge[2] + (l.third[2] - l.edge[2]) * g;
            const al = a * l.op * (0.3 + 0.7 * exp(-sdp / l.fade));
            c0 += (m0 + (l.deep[0] - m0) * t + hi - c0) * al;
            c1 += (m1 + (l.deep[1] - m1) * t - c1) * al;
            c2 += (m2 + (l.deep[2] - m2) * t - c2) * al;
          }
        }
        const n = gr * ((hash2(x, y, S) + hash2(x, y, S + 1) - 1) * 1.4);
        const rgb = oklab2lin(c0 + n * (0.6 + 0.4 * c0), c1 * 1.2, c2 * 1.2);
        put(d, y * W + x, enc(rgb[0]), enc(rgb[1]), enc(rgb[2]));
      }
    }
    return d;
  },
};
