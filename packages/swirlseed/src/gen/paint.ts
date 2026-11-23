/** Paint on paper: watercolour, wet-on-wet, paste paper and dendritic painting. */
import { TAU, blur, clamp, cos, developPaper, enc, exp, fbm2, hash2, hypot2, hypot3, lin, lin3, ln, newImage, paperHeight, paperLight, pigmentLayer, pow, put, round, sin, sstep, stream, vnoise, type V3 } from "./base.js";
import { Mask, capsule, disc, fillPolygon } from "./raster.js";
import { type Generator, num, opt, str } from "./spec.js";

type Pal = [V3, number][];
const mkPal = (a: [V3, number][]): Pal => a.map(([c, g]) => [lin3(c), g]);

// ---------------- watercolour ----------------
const WC: Record<string, [V3, number][]> = {
  cobalt: [[[40, 80, 170], 0.8], [[20, 120, 150], 0.5], [[70, 60, 150], 0.9], [[200, 120, 60], 0.6], [[30, 90, 80], 0.3]],
  sunset: [[[220, 70, 50], 0.3], [[240, 150, 40], 0.2], [[200, 50, 110], 0.3], [[110, 60, 140], 0.8], [[250, 200, 80], 0.1]],
  forest: [[[50, 110, 60], 0.4], [[140, 150, 50], 0.3], [[30, 80, 90], 0.6], [[150, 100, 50], 0.8], [[90, 120, 40], 0.4]],
  rose: [[[210, 70, 110], 0.3], [[230, 120, 120], 0.2], [[140, 70, 150], 0.6], [[240, 170, 90], 0.2], [[90, 110, 170], 0.7]],
  sepia: [[[110, 70, 40], 0.8], [[150, 100, 60], 0.7], [[70, 50, 40], 0.9], [[180, 140, 90], 0.5], [[80, 90, 110], 0.8]],
};

type Poly = number[]; // x0,y0,x1,y1,…
function deform(R: () => number, p: Poly, depth: number, vr: number): Poly {
  const gauss = (): number => {
    let u = 0;
    while (u === 0) u = R();
    return Math.sqrt(-2 * ln(u)) * cos(TAU * R());
  };
  for (let d = 0; d < depth; d++) {
    const q: Poly = [];
    const n = p.length / 2;
    for (let i = 0; i < n; i++) {
      const j = (i + 1) % n;
      const ax = p[i * 2];
      const ay = p[i * 2 + 1];
      const bx = p[j * 2];
      const by = p[j * 2 + 1];
      const l = hypot2(bx - ax, by - ay);
      const gx = gauss();
      const gy = gauss();
      q.push(ax, ay, (ax + bx) / 2 + gx * vr * l, (ay + by) / 2 + gy * vr * l);
    }
    p = q;
  }
  return p;
}
function blob(R: () => number, cx: number, cy: number, rx: number, ry: number, n: number): Poly {
  const p: Poly = [];
  const ph = R() * TAU;
  for (let i = 0; i < n; i++) {
    const a = ph + (i / n) * TAU;
    const rr = 0.75 + 0.5 * R();
    p.push(cx + cos(a) * rx * rr, cy + sin(a) * ry * rr);
  }
  return p;
}
/** Source-over alpha accumulation of one polygon into A. */
function layer(m: Mask, A: Float64Array, p: Poly, alpha: number): void {
  const n = p.length / 2;
  const xs: number[] = [];
  const ys: number[] = [];
  for (let i = 0; i < n; i++) {
    xs.push(p[i * 2]);
    ys.push(p[i * 2 + 1]);
  }
  fillPolygon(m, xs, ys, n);
  accumulate(m, A, alpha);
}
function accumulate(m: Mask, A: Float64Array, alpha: number): void {
  for (let y = m.y0; y <= m.y1; y++)
    for (let x = m.x0; x <= m.x1; x++) {
      const i = y * m.W + x;
      let c = m.cov[i];
      if (c <= 0) continue;
      if (c > 1) c = 1;
      A[i] += c * alpha * (1 - A[i]);
      m.cov[i] = 0;
    }
  m.x0 = 0;
  m.y0 = 0;
  m.x1 = -1;
  m.y1 = -1;
}

export const watercolor: Generator = {
  id: "watercolor",
  title: "Watercolor",
  description: "Transparent washes on cold-press paper: edge pooling, granulation, glazing, blooms and splatter.",
  aspect: 1.5,
  params: [
    { kind: "choice", key: "palette", label: "Palette", default: "cobalt", options: [opt("cobalt", "Cobalt"), opt("sunset", "Sunset"), opt("forest", "Forest"), opt("rose", "Rose"), opt("sepia", "Sepia")] },
    { kind: "number", key: "washes", label: "Washes", min: 2, max: 12, step: 1, default: 6 },
    { kind: "number", key: "layers", label: "Layers", min: 10, max: 40, step: 1, default: 24 },
    { kind: "number", key: "bleed", label: "Bleed", min: 0, max: 1, step: 0.01, default: 0.5, unit: "%" },
    { kind: "number", key: "splatter", label: "Splatter", min: 0, max: 1, step: 0.01, default: 0.35, unit: "%" },
  ],
  previews: [
    { title: "Palette", sub: "Five pigment sets, same seed.", key: "palette", values: ["cobalt", "sunset", "forest", "rose", "sepia"], labels: ["Cobalt", "Sunset", "Forest", "Rose", "Sepia"] },
    { title: "Washes", sub: "From one quiet stain to a busy sheet.", key: "washes", values: [2, 5, 8, 12], labels: ["2", "5", "8", "12"] },
  ],
  render(seed, W, H, p) {
    const R = stream(seed, "watercolor");
    const S = Math.floor(R() * 1e6);
    const m = Math.min(W, H);
    const pal = mkPal(WC[str(p, "palette")]);
    const ph = paperHeight(W, H, S, 1);
    const pl = paperLight(ph, W, H, 1.1);
    const Ab = new Float64Array(W * H * 3);
    const mask = new Mask(W, H);
    const nW = round(num(p, "washes"));
    const layers = round(num(p, "layers"));
    const bleed = num(p, "bleed");
    for (let w = 0; w < nW; w++) {
      const big = w < 2;
      const cx = W * (0.1 + 0.8 * R());
      const cy = H * (0.1 + 0.8 * R());
      const r = m * (big ? 0.3 + 0.15 * R() : 0.1 + 0.17 * R());
      const [pig, gr] = pal[Math.floor(R() * pal.length)];
      const A = new Float64Array(W * H);
      const rx = r * (1 + 0.5 * R());
      const ry = r * (0.7 + 0.4 * R());
      const base = deform(R, blob(R, cx, cy, rx, ry, 10), 3, 0.22);
      const la = 1.6 / layers;
      for (let l = 0; l < layers; l++) layer(mask, A, deform(R, base, 3, 0.1 + 0.06 * R()), la);
      const ns = round(num(p, "splatter") * 40 * (big ? 0.5 : 1));
      for (let s = 0; s < ns; s++) {
        const a = R() * TAU;
        const dd = r * (0.9 + R() * 1.3);
        const q = R();
        const sr = m * 0.0022 * (0.4 + q * q * 4);
        disc(mask, cx + cos(a) * dd, cy + sin(a) * dd * 0.8, sr);
        accumulate(mask, A, 0.42);
      }
      for (let i = 0; i < W * H; i++) A[i] = Math.min(1, A[i] * 1.05);
      const edge = 2.2 + 1.6 * R();
      const mottleF = 2 + 3 * R();
      const bloom = R() < bleed * 0.8 ? 0.8 : 0;
      const bloomF = 4 + 4 * R();
      const bloomT = 0.56 + 0.06 * R();
      const strength = big ? 0.55 + 0.4 * R() : 0.8 + 0.6 * R();
      pigmentLayer(Ab, A, W, H, pig, { S: S + w * 97, ph, gran: gr * 0.8, edge, edgeR: 0.005, mottleF, bloom, bloomF, bloomT, strength });
      if (R() < bleed) {
        const B = new Float64Array(W * H);
        const bx = cx + (R() - 0.5) * r * 1.6;
        const by = cy + (R() - 0.5) * r * 1.6;
        for (let l = 0; l < 10; l++) layer(mask, B, deform(R, blob(R, bx, by, r * 0.5, r * 0.45, 8), 4, 0.2), 0.5);
        const br = round(m * 0.02);
        const Bm = blur(blur(B, W, H, br), W, H, br);
        const [p2, g2] = pal[Math.floor(R() * pal.length)];
        pigmentLayer(Ab, Bm, W, H, p2, { S: S + w * 31 + 7, ph, gran: g2 * 0.8, edge: 0.6, edgeR: 0.006, mottleF: 3, bloom: 0, bloomF: 5, bloomT: 0.58, strength: 0.6 });
      }
    }
    return developPaper(Ab, W, H, lin3([247, 243, 233]), pl);
  },
};

// ---------------- wet on wet ----------------
const WW: Record<string, [V3, number][]> = {
  dusk: [[[60, 50, 150], 0.8], [[200, 70, 110], 0.3], [[235, 140, 70], 0.2], [[40, 100, 160], 0.6], [[245, 200, 90], 0.1]],
  ocean: [[[20, 70, 150], 0.8], [[20, 140, 160], 0.5], [[80, 170, 170], 0.4], [[30, 40, 110], 0.9], [[120, 180, 200], 0.3]],
  bloom: [[[215, 50, 100], 0.2], [[240, 120, 70], 0.2], [[140, 60, 160], 0.5], [[250, 190, 70], 0.1], [[70, 90, 180], 0.6]],
  earth: [[[150, 80, 40], 0.8], [[190, 130, 50], 0.6], [[80, 90, 50], 0.5], [[110, 50, 40], 0.8], [[60, 70, 100], 0.9]],
};

export const wetOnWet: Generator = {
  id: "wet-on-wet",
  title: "Wet on wet",
  description: "Pigment dropped onto soaking paper: feathered fronts, mixing washes, granulation and cauliflower blooms.",
  aspect: 1.5,
  params: [
    { kind: "choice", key: "palette", label: "Palette", default: "dusk", options: [opt("dusk", "Dusk"), opt("ocean", "Ocean"), opt("bloom", "Bloom"), opt("earth", "Earth")] },
    { kind: "number", key: "washes", label: "Washes", min: 2, max: 8, step: 1, default: 5 },
    { kind: "number", key: "flow", label: "Flow", min: 0, max: 1, step: 0.01, default: 0.55, unit: "%" },
    { kind: "number", key: "water", label: "Water", min: 0, max: 1, step: 0.01, default: 0.55, unit: "%" },
    { kind: "number", key: "bloom", label: "Bloom", min: 0, max: 1, step: 0.01, default: 0.45, unit: "%" },
  ],
  previews: [
    { title: "Palette", sub: "Four pigment families.", key: "palette", values: ["dusk", "ocean", "bloom", "earth"], labels: ["Dusk", "Ocean", "Bloom", "Earth"] },
    { title: "Water", sub: "Damp and crisp to fully flooded.", key: "water", values: [0.1, 0.5, 0.8, 1], labels: ["Damp", "Wet", "Soaked", "Flooded"] },
  ],
  render(seed, W, H, p) {
    const R = stream(seed, "wet-on-wet");
    const S = Math.floor(R() * 1e6);
    const asp = W / H;
    const pal = mkPal(WW[str(p, "palette")]);
    const n = round(num(p, "washes"));
    const flow = num(p, "flow");
    const water = num(p, "water");
    const bloomP = num(p, "bloom");
    const ph = paperHeight(W, H, S, 1);
    const pl = paperLight(ph, W, H, 1.1);
    const Ab = new Float64Array(W * H * 3);
    const wx = R() * asp;
    const wy = R();
    const wetR = 0.5 + 0.6 * water;
    for (let k = 0; k < n; k++) {
      const cx = asp * (0.08 + 0.84 * R());
      const cy = 0.08 + 0.84 * R();
      const r = 0.18 + 0.24 * R() + (k < 2 ? 0.12 : 0);
      const [pig, gr] = pal[Math.floor(R() * pal.length)];
      const ox = R() * 50;
      const oy = R() * 50;
      const soft = 0.03 + 0.16 * water;
      const A = new Float64Array(W * H);
      const sharp = R() < 0.5;
      for (let y = 0; y < H; y++) {
        const v = y / H;
        for (let x = 0; x < W; x++) {
          const u = x / H;
          const wu = (fbm2(u * 1.3 + ox, v * 1.3 + oy, S + k, 4) - 0.5) * flow * 0.6;
          const wv = (fbm2(u * 1.3 + oy, v * 1.3 + ox, S + k + 40, 4) - 0.5) * flow * 0.6;
          const dx = (u + wu - cx) / r;
          const dy = (v + wv - cy) / r;
          const q = Math.sqrt(dx * dx + dy * dy);
          const nz = fbm2(u * 3 + ox, v * 3 + oy, S + k * 7, 6);
          const wet = clamp(1 - (hypot2(u - wx, v - wy) / wetR / asp) * 1.4, 0, 1);
          const f = 1 - q + (nz - 0.5) * (0.7 + 0.6 * flow);
          const s = soft * (0.4 + 1.6 * wet) * (sharp ? 0.6 : 1);
          A[y * W + x] = sstep(-s, s * 0.6, f) * (0.55 + 0.45 * sstep(0, 0.5, f));
        }
      }
      const bloom = R() < bloomP ? 1 : 0;
      const bloomF = 3 + 3 * R();
      const bloomT = 0.55 + 0.08 * R();
      const strength = 0.6 + 0.5 * R();
      pigmentLayer(Ab, A, W, H, pig, { S: S + k * 17, ph, gran: gr * 0.55, edge: bloomP * 2.6 * (sharp ? 1 : 0.4), edgeR: 0.004 + 0.006 * water, mottleF: 2.5, bloom, bloomF, bloomT, strength });
    }
    return developPaper(Ab, W, H, lin3([248, 244, 234]), pl);
  },
};

// ---------------- paste paper ----------------
const PC: Record<string, [V3, V3]> = {
  indigo: [[24, 40, 110], [236, 230, 214]],
  terracotta: [[160, 58, 34], [242, 228, 204]],
  forest: [[26, 84, 62], [236, 232, 210]],
  plum: [[92, 32, 86], [240, 226, 222]],
  ochre: [[190, 130, 30], [244, 236, 220]],
};
const G = new Float64Array(2);
function groove(f: number, jit: number): void {
  const q = Math.abs(f - Math.floor(f) - 0.5);
  const gw = 0.16 + jit;
  G[0] = 1 - sstep(gw - 0.05, gw + 0.03, 0.5 - q);
  const rd = (0.5 - q - gw - 0.04) / 0.05;
  G[1] = exp(-rd * rd);
}

export const pastePaper: Generator = {
  id: "paste-paper",
  title: "Paste paper",
  description: "Bookbinder's endpaper: coloured starch paste combed, swirled or stamped while wet, lit as a relief.",
  aspect: 1.5,
  params: [
    { kind: "choice", key: "pattern", label: "Pattern", default: "waves", options: [opt("waves", "Waves"), opt("basket", "Basket"), opt("rings", "Rings"), opt("stamped", "Stamped")] },
    { kind: "choice", key: "colour", label: "Colour", default: "indigo", options: [opt("indigo", "Indigo"), opt("terracotta", "Terracotta"), opt("forest", "Forest"), opt("plum", "Plum"), opt("ochre", "Ochre")] },
    { kind: "number", key: "comb", label: "Comb", min: 3, max: 22, step: 0.5, default: 10 },
    { kind: "number", key: "wave", label: "Wave", min: 0, max: 1.5, step: 0.01, default: 0.8, unit: "%" },
    { kind: "number", key: "streak", label: "Brush", min: 0, max: 1, step: 0.01, default: 0.5, unit: "%" },
  ],
  previews: [
    { title: "Pattern", sub: "Wavy comb, basket weave, combed rings and cork stamps.", key: "pattern", values: ["waves", "basket", "rings", "stamped"], labels: ["Waves", "Basket", "Rings", "Stamped"] },
    { title: "Colour", sub: "Five pigments over cream paper.", key: "colour", values: ["indigo", "terracotta", "forest", "plum", "ochre"], labels: ["Indigo", "Terracotta", "Forest", "Plum", "Ochre"] },
  ],
  render(seed, W, H, p) {
    const R = stream(seed, "paste-paper");
    const S = Math.floor(R() * 1e6);
    const m = Math.min(W, H);
    const pc = PC[str(p, "colour")];
    const pig = lin3(pc[0]);
    const paper = lin3(pc[1]);
    const pat = str(p, "pattern");
    const F = num(p, "comb");
    const A = num(p, "wave");
    const streak = num(p, "streak");
    const ox = R() * 40;
    R();
    const ph = R() * TAU;
    const ph2 = R() * TAU;
    const rot = (R() - 0.5) * 0.35;
    const cr = cos(rot);
    const sr = sin(rot);
    const cxs: number[] = [];
    const cys: number[] = [];
    for (let k = 0; k < 5; k++) {
      cxs.push((R() * W) / m);
      cys.push((R() * H) / m);
    }
    const T = new Float64Array(W * H);
    for (let y = 0; y < H; y++)
      for (let x = 0; x < W; x++) {
        const u = x / m;
        const v = y / m;
        const bx = u * cr + v * sr;
        const by = -u * sr + v * cr;
        const brush = 0.85 + 0.42 * (vnoise(bx * 2.5 + ox, by * 90, S) * 0.6 + vnoise(bx * 9, by * 260, S + 1) * 0.4 - 0.5) * streak * 2;
        const jit = (fbm2(u * 6, v * 6, S + 3, 3) - 0.5) * 0.14 + (fbm2(u * 30, v * 30, S + 4, 2) - 0.5) * 0.05;
        let g1 = 0;
        let r1 = 0;
        let phase = 0;
        if (pat === "waves") {
          phase = (by + A * (0.06 * sin(bx * 7 + ph) + 0.035 * sin(bx * 15 + ph2)) + (fbm2(u * 1.5, v * 1.5, S + 5, 3) - 0.5) * A * 0.12) * F;
          groove(phase, jit);
          g1 = G[0];
          r1 = G[1];
        } else if (pat === "basket") {
          const p1 = (bx + A * 0.03 * sin(by * 9 + ph)) * F;
          const p2 = (by + A * 0.03 * sin(bx * 9 + ph2)) * F;
          const blk = Math.floor((bx * F) / 4) + Math.floor((by * F) / 4);
          phase = (blk & 1) !== 0 ? p1 : p2;
          groove(phase, jit);
          g1 = G[0];
          r1 = G[1];
        } else if (pat === "rings") {
          let s = 0;
          for (let k = 0; k < 5; k++) s += exp(-hypot2(u - cxs[k], v - cys[k]) * 16);
          const d = -ln(s + 1e-9) / 16 + (fbm2(u * 3, v * 3, S + 7, 3) - 0.5) * A * 0.1;
          phase = d * F * 1.4;
          groove(phase, jit);
          g1 = G[0];
          r1 = G[1];
        } else {
          const g = F * 0.5;
          const uu = bx * g + (fbm2(u * 2, v * 2, S + 8, 2) - 0.5) * A;
          const vv = by * g + (fbm2(u * 2 + 5, v * 2, S + 9, 2) - 0.5) * A;
          const fx = Math.floor(uu);
          const fy = Math.floor(vv);
          let dm = 9;
          for (let j = -1; j <= 1; j++)
            for (let q = -1; q <= 1; q++) {
              const px = fx + q + 0.25 + 0.5 * hash2(fx + q, fy + j, S);
              const py = fy + j + 0.25 + 0.5 * hash2(fx + q, fy + j, S + 3);
              const dd = hypot2(uu - px, vv - py);
              if (dd < dm) dm = dd;
            }
          const rr = 0.27 + jit;
          const inner = 1 - sstep(rr - 0.03, rr + 0.01, dm);
          const vein = fbm2(u * 60, v * 60, S + 12, 3);
          g1 = inner * (0.55 + 0.45 * sstep(0.4, 0.62, vein));
          const z = (dm - rr - 0.035) / 0.03;
          r1 = exp(-z * z);
        }
        const sq = 0.5 + 0.5 * sin(phase * TAU * 7 + fbm2(u * 20, v * 20, S + 13, 2) * 4);
        const st = sq * sq * (0.5 + vnoise(phase * 40, v * 3, S + 14));
        const t = brush * (1 - 0.8 * g1) + 0.38 * r1 + g1 * 0.22 * st;
        T[y * W + x] = Math.max(0, t);
      }
    const Tb = blur(T, W, H, Math.max(1, round(m * 0.0012)));
    const pH = paperHeight(W, H, S, 1);
    const pl = paperLight(pH, W, H, 0.8);
    const k = [-ln(pig[0]), -ln(pig[1]), -ln(pig[2])];
    const d = newImage(W, H);
    for (let y = 0; y < H; y++)
      for (let x = 0; x < W; x++) {
        const i = y * W + x;
        const t = Tb[i];
        const xm = x > 0 ? Tb[i - 1] : t;
        const ym = y > 0 ? Tb[i - W] : t;
        const xp = x < W - 1 ? Tb[i + 1] : t;
        const yp = y < H - 1 ? Tb[i + W] : t;
        const nx = (xm - xp) * 3.5;
        const ny = (ym - yp) * 3.5;
        const nl = hypot3(nx, ny, 1);
        const lam = 0.86 + ((-nx * 0.6 - ny * 0.75) / nl) * 0.5;
        const sh = pow(clamp((-nx * 0.5 - ny * 0.6 + 0.62) / nl, 0, 1), 40) * 0.08 * t;
        const D = t * 0.82 * (0.9 + 0.2 * pH[i]) * (0.85 + 0.3 * fbm2((x / m) * 4, (y / m) * 4, S + 20, 4));
        put(d, i, enc((paper[0] * exp(-k[0] * D) * lam + sh) * pl[i]), enc((paper[1] * exp(-k[1] * D) * lam + sh) * pl[i]), enc((paper[2] * exp(-k[2] * D) * lam + sh) * pl[i]));
      }
    return d;
  },
};

// ---------------- dendritic painting ----------------
const DP: Record<string, { paper: V3; inks: V3[]; dark: boolean }> = {
  ink: { paper: [244, 238, 226], inks: [[22, 20, 24], [40, 30, 26]], dark: false },
  cobalt: { paper: [246, 244, 238], inks: [[16, 48, 140], [20, 120, 150], [60, 30, 120]], dark: false },
  ember: { paper: [18, 16, 20], inks: [[255, 120, 30], [255, 200, 60], [220, 40, 50]], dark: true },
  spectrum: { paper: [246, 243, 236], inks: [[230, 40, 90], [250, 170, 30], [20, 150, 170], [60, 60, 170]], dark: false },
  malachite: { paper: [238, 234, 222], inks: [[10, 90, 70], [30, 140, 100], [10, 50, 60]], dark: false },
};

export const dendritic: Generator = {
  id: "dendritic-painting",
  title: "Dendritic painting",
  description: "Paint pressed between plates and pulled apart: branching fingers grown by diffusion-limited aggregation.",
  aspect: 1.5,
  params: [
    { kind: "choice", key: "palette", label: "Paint", default: "cobalt", options: [opt("ink", "Ink"), opt("cobalt", "Cobalt"), opt("ember", "Ember"), opt("spectrum", "Spectrum"), opt("malachite", "Malachite")] },
    { kind: "choice", key: "origin", label: "Origin", default: "edges", options: [opt("edges", "Edges"), opt("centre", "Centre"), opt("scatter", "Scatter")] },
    { kind: "number", key: "fill", label: "Growth", min: 0.2, max: 1, step: 0.01, default: 0.85, unit: "%" },
    { kind: "number", key: "branch", label: "Branching", min: 0, max: 1, step: 0.01, default: 0.7, unit: "%" },
    { kind: "number", key: "weight", label: "Weight", min: 0.6, max: 2, step: 0.02, default: 1, unit: "x" },
  ],
  previews: [
    { title: "Paint", sub: "Five paint sets, same growth.", key: "palette", values: ["ink", "cobalt", "ember", "spectrum", "malachite"], labels: ["Ink", "Cobalt", "Ember", "Spectrum", "Malachite"] },
    { title: "Origin", sub: "From the edges, from the centre, or scattered seeds.", key: "origin", values: ["edges", "centre", "scatter"], labels: ["Edges", "Centre", "Scatter"] },
  ],
  render(seed, W, H, p) {
    const R = stream(seed, "dendritic");
    const S = Math.floor(R() * 1e6);
    const pal = DP[str(p, "palette")];
    const cs = Math.max(2, W / 260);
    const gw = round(W / cs);
    const gh = round(H / cs);
    const N = gw * gh;
    const occ = new Int32Array(N).fill(-1);
    const near = new Uint8Array(N);
    const px: number[] = [];
    const py: number[] = [];
    const par: number[] = [];
    const col: number[] = [];
    const BS = 8;
    const bw = Math.ceil(gw / BS);
    const bh = Math.ceil(gh / BS);
    const bnear = new Uint8Array(bw * bh);
    const occupy = (i: number, pr: number, c: number): void => {
      const x = i % gw;
      const y = Math.floor(i / gw);
      const bx = Math.floor(x / BS);
      const by = Math.floor(y / BS);
      for (let j = -1; j <= 1; j++)
        for (let q = -1; q <= 1; q++) {
          const X = bx + q;
          const Y = by + j;
          if (X >= 0 && Y >= 0 && X < bw && Y < bh) bnear[Y * bw + X] = 1;
        }
      occ[i] = px.length;
      px.push(x);
      py.push(y);
      par.push(pr);
      col.push(c);
      if (x > 0) near[i - 1] = 1;
      if (x < gw - 1) near[i + 1] = 1;
      if (y > 0) near[i - gw] = 1;
      if (y < gh - 1) near[i + gw] = 1;
    };
    const org = str(p, "origin");
    const nI = pal.inks.length;
    if (org === "edges") {
      const n = 3 + Math.floor(R() * 4);
      for (let s = 0; s < n; s++) {
        const e = Math.floor(R() * 4);
        const t = 0.1 + 0.8 * R();
        let x: number;
        let y: number;
        if (e === 0) {
          x = t * gw;
          y = 0;
        } else if (e === 1) {
          x = t * gw;
          y = gh - 1;
        } else if (e === 2) {
          x = 0;
          y = t * gh;
        } else {
          x = gw - 1;
          y = t * gh;
        }
        const i = Math.floor(y) * gw + Math.floor(x);
        if (occ[i] < 0) occupy(i, -1, s % nI);
      }
    } else {
      const n = org === "centre" ? 1 + Math.floor(R() * 3) : 6 + Math.floor(R() * 8);
      const lo = org === "centre" ? 0.35 : 0.05;
      const sp = org === "centre" ? 0.3 : 0.9;
      const lox = org === "centre" ? 0.3 : 0.05;
      const spx = org === "centre" ? 0.4 : 0.9;
      for (let s = 0; s < n; s++) {
        const yy = Math.floor(gh * (lo + sp * R()));
        const xx = Math.floor(gw * (lox + spx * R()));
        const i = yy * gw + xx;
        if (occ[i] < 0) occupy(i, -1, s % nI);
      }
    }
    const target = round(N * num(p, "fill") * 0.16);
    const stick = 0.3 + 0.7 * num(p, "branch");
    let guard = 0;
    while (px.length < target && guard++ < target * 40) {
      let x = Math.floor(R() * gw);
      let y = Math.floor(R() * gh);
      let i = y * gw + x;
      if (occ[i] >= 0) continue;
      for (let s = 0; s < 9000; s++) {
        if (near[i] && R() < stick) {
          let pr = -1;
          const cand = [i - 1, i + 1, i - gw, i + gw];
          for (const j of cand) {
            if (j >= 0 && j < N && occ[j] >= 0 && Math.abs((j % gw) - x) <= 1) {
              pr = occ[j];
              break;
            }
          }
          if (pr >= 0) {
            occupy(i, pr, col[pr]);
            break;
          }
        }
        if (!bnear[Math.floor(y / BS) * bw + Math.floor(x / BS)]) {
          const a = R() * TAU;
          x = round(x + cos(a) * 6);
          y = round(y + sin(a) * 6);
        } else {
          const r = Math.floor(R() * 4);
          if (r === 0) x++;
          else if (r === 1) x--;
          else if (r === 2) y++;
          else y--;
        }
        if (x < 0 || y < 0 || x >= gw || y >= gh) break;
        i = y * gw + x;
        if (occ[i] >= 0) break;
      }
    }
    const n = px.length;
    const cnt = new Float64Array(n).fill(1);
    for (let k = n - 1; k >= 0; k--) if (par[k] >= 0) cnt[par[k]] += cnt[k];
    const jx = (k: number): number => (px[k] + 0.5 + (hash2(px[k], py[k], S + 1) - 0.5) * 0.95) * cs;
    const jy = (k: number): number => (py[k] + 0.5 + (hash2(px[k], py[k], S + 2) - 0.5) * 0.95) * cs;
    const th = new Float64Array(W * H);
    const cw: Float64Array[] = [];
    const mask = new Mask(W, H);
    const wt = num(p, "weight");
    for (let c = 0; c < nI; c++) {
      const t = new Float64Array(W * H);
      for (let k = 0; k < n; k++) {
        if (col[k] !== c || par[k] < 0) continue;
        const q = par[k];
        capsule(mask, jx(k), jy(k), jx(q), jy(q), cs * (0.45 + 0.42 * ln(1 + cnt[k])) * wt);
      }
      for (let y = mask.y0; y <= mask.y1; y++)
        for (let x = mask.x0; x <= mask.x1; x++) {
          const i = y * W + x;
          t[i] = Math.min(1, mask.cov[i]);
          th[i] += t[i];
          mask.cov[i] = 0;
        }
      mask.x0 = 0;
      mask.y0 = 0;
      mask.x1 = -1;
      mask.y1 = -1;
      cw.push(t);
    }
    const br = Math.max(1, round(cs * 0.8));
    const tb = blur(blur(th, W, H, br), W, H, br);
    const ph = paperHeight(W, H, S, 1);
    const pl = paperLight(ph, W, H, 0.9);
    const pp = lin3(pal.paper);
    const inks = pal.inks.map(lin3);
    const stain = blur(th, W, H, round(cs * 5));
    const d = newImage(W, H);
    for (let y = 0; y < H; y++)
      for (let x = 0; x < W; x++) {
        const i = y * W + x;
        let r = pp[0] * pl[i];
        let g = pp[1] * pl[i];
        let b = pp[2] * pl[i];
        const sum = th[i];
        const cov = clamp(sum, 0, 1);
        const halo = clamp(stain[i] * 1.2, 0, 1) * (1 - cov) * 0.12;
        if (!pal.dark) {
          r *= 1 - halo * 0.6;
          g *= 1 - halo * 0.7;
          b *= 1 - halo * 0.95;
        }
        if (cov > 0) {
          let ir = 0;
          let ig = 0;
          let ib = 0;
          for (let c = 0; c < nI; c++) {
            const w = cw[c][i] / (sum + 1e-6);
            ir += inks[c][0] * w;
            ig += inks[c][1] * w;
            ib += inks[c][2] * w;
          }
          const thick = clamp(tb[i] * 1.15, 0, 1.4);
          const edge = clamp(cov - tb[i] * 0.9, 0, 1);
          const xm = x > 0 ? tb[i - 1] : tb[i];
          const ym = y > 0 ? tb[i - W] : tb[i];
          const xp = x < W - 1 ? tb[i + 1] : tb[i];
          const yp = y < H - 1 ? tb[i + W] : tb[i];
          const nx = (xm - xp) * 1.6;
          const ny = (ym - yp) * 1.6;
          const nl = hypot3(nx, ny, 1);
          const lam = clamp(0.78 + ((-nx * 0.55 - ny * 0.7) / nl) * 0.6, 0, 1.4);
          const spec = pow(clamp((-nx * 0.4 - ny * 0.6 + 0.7) / nl, 0, 1), 28) * 0.35 * thick;
          const dens = 0.55 + 0.45 * thick + edge * 0.35;
          const gran = 0.9 + 0.2 * ph[i];
          let pr: number;
          let pg: number;
          let pb: number;
          if (pal.dark) {
            const f = lam * gran * (1.1 - 0.3 * thick);
            pr = ir * f;
            pg = ig * f;
            pb = ib * f;
          } else {
            pr = pp[0] * pow(ir, dens * gran) * lam;
            pg = pp[1] * pow(ig, dens * gran) * lam;
            pb = pp[2] * pow(ib, dens * gran) * lam;
          }
          r = r * (1 - cov) + (pr + spec) * cov;
          g = g * (1 - cov) + (pg + spec) * cov;
          b = b * (1 - cov) + (pb + spec) * cov;
        }
        put(d, i, enc(r), enc(g), enc(b));
      }
    return d;
  },
};
// keep linters quiet about helpers used only by some generators
void lin;
