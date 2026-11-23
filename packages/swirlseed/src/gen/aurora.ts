/** Aurora: folded emitting sheets 100–300 km up, seen in perspective from the ground, over stars. */
import { blur, clamp, cos, enc, exp, fbm2, hash2, lerp, newImage, pow, put, round, sin, sstep, stream, vnoise, type V3 } from "./base.js";
import { type Generator, num, opt, str } from "./spec.js";

const COL: Record<string, { fr: V3; lo: V3; hi: V3; top: V3 }> = {
  green: { fr: [0.5, 0.9, 0.6], lo: [0.12, 1, 0.38], hi: [0.08, 0.55, 0.42], top: [0.55, 0.12, 0.35] },
  violet: { fr: [0.9, 0.5, 1], lo: [0.55, 0.3, 1], hi: [0.3, 0.35, 1], top: [0.25, 0.15, 0.7] },
  crimson: { fr: [1, 0.5, 0.5], lo: [1, 0.16, 0.24], hi: [0.8, 0.12, 0.4], top: [0.4, 0.08, 0.4] },
  mixed: { fr: [1, 0.3, 0.7], lo: [0.15, 1, 0.45], hi: [0.1, 0.7, 0.5], top: [0.9, 0.15, 0.35] },
};

export const aurora: Generator = {
  id: "aurora",
  title: "Aurora",
  description: "Northern lights: thin glowing curtains that fold back on themselves, in perspective over a starry sky.",
  aspect: 1.6,
  params: [
    { kind: "choice", key: "colour", label: "Colour", default: "green", options: [opt("green", "Green"), opt("violet", "Violet"), opt("crimson", "Crimson"), opt("mixed", "Mixed")] },
    { kind: "number", key: "intensity", label: "Intensity", min: 0.4, max: 2, step: 0.02, default: 1, unit: "%" },
    { kind: "number", key: "curtains", label: "Curtains", min: 1, max: 3, step: 1, default: 2 },
    { kind: "number", key: "rays", label: "Rays", min: 20, max: 140, step: 1, default: 70 },
    { kind: "number", key: "stars", label: "Stars", min: 0, max: 1, step: 0.01, default: 0.7, unit: "%" },
  ],
  previews: [
    { title: "Colour", sub: "Oxygen green, violet, crimson, and a mixed display.", key: "colour", values: ["green", "violet", "crimson", "mixed"], labels: ["Green", "Violet", "Crimson", "Mixed"] },
    { title: "Curtains", sub: "One lone arc up to a layered sky.", key: "curtains", values: [1, 2, 3], labels: ["1", "2", "3"] },
  ],
  render(seed, W, H, p) {
    const R = stream(seed, "aurora");
    const S = Math.floor(R() * 1e6);
    const C = COL[str(p, "colour")];
    const nc = round(num(p, "curtains"));
    const rays = num(p, "rays");
    const inten = num(p, "intensity");
    const N = W * H;
    const Er = new Float64Array(N);
    const Eg = new Float64Array(N);
    const Eb = new Float64Array(N);
    const f = H * 0.85;
    const hor = H * 1.04;
    const cx = W / 2;
    for (let c = 0; c < nc; c++) {
      const off = R() * 50;
      const Zc = c === 0 ? 110 + R() * 90 : 220 + R() * 500;
      const phi0 = (R() - 0.5) * 0.8;
      const bend = 1.6 + 1.8 * R();
      const len = Zc * (3.2 + 1.6 * R());
      const M = round(W * 14);
      const top = 70 + 120 * R();
      const curl = 6 + 10 * R();
      const near = 320 / Zc;
      const XA = new Float64Array(M);
      const ZA = new Float64Array(M);
      {
        let X = 0;
        let Z = Zc;
        let sx = 0;
        for (let k = 0; k < M; k++) {
          const ph = phi0 + bend * (fbm2((k / M) * 2.2 + off, c, S + c, 3) - 0.5) * 2;
          X += (cos(ph) * len) / M;
          Z += ((sin(ph) * len) / M) * 0.6;
          XA[k] = X;
          ZA[k] = Math.max(Z, 30);
          if (Z >= 100) sx += X / Z;
        }
        const c0 = sx / M - (R() - 0.5) * 0.35;
        for (let k = 0; k < M; k++) XA[k] -= c0 * ZA[k];
      }
      for (let k = 0; k < M; k++) {
        const s = k / M;
        const X = XA[k];
        const Z = ZA[k];
        if (Z < 100) continue;
        const Xc = X + curl * (fbm2(s * 38 + off, c + 3, S + c + 5, 3) - 0.5) * 2 * (0.4 + fbm2(s * 4 + off, c, S + c + 6, 2));
        const x = cx + (Xc / Z) * f;
        if (x < -2 || x >= W + 1) continue;
        const a0 = 100 + 8 * (fbm2(s * 9 + off, c + 1, S + c + 7, 2) - 0.5);
        const a1 = a0 + top * (0.45 + 0.9 * fbm2(s * 6 + off, c + 2, S + c + 8, 3)) * (0.7 + 0.6 * vnoise(s * rays * 3 + off, c, S + c + 9));
        const yb = hor - (a0 * f) / Z;
        const h = ((a1 - a0) * f) / Z;
        const ray = pow(clamp(vnoise(s * rays * 4 + off, c, S + c + 60) * 0.6 + vnoise(s * rays * 13 + off, c + 0.5, S + c + 61) * 0.4, 0, 1), 2.4);
        const act = pow(clamp(fbm2(s * 3.2 + off, c + 7, S + c + 80, 4) * 2 - 0.35, 0, 1), 1.1) * sstep(0, 0.08, s) * sstep(1, 0.92, s);
        const ext = exp(-Math.max(0, Z - 300) / 1400);
        const I = (0.12 + 2 * ray) * act * inten * ext * ((len / M / Z) * f * 0.3) * (near > 1 ? 1 : 0.75 + 0.25 * near);
        if (I < 1e-6) continue;
        const xi = Math.floor(x);
        const fx = x - xi;
        const y0 = Math.max(0, Math.floor(yb - h * 1.7));
        const y1 = Math.min(H - 1, Math.ceil(yb + 3));
        for (let y = y0; y <= y1; y++) {
          const z = (yb - y) / h;
          let e: number;
          if (z < 0) e = exp(z * h * 0.8);
          else e = (exp(-z * 2.6) * 0.85 + 0.15 * exp(-z * 0.7)) * (1 - exp(-z * h * 0.45 - 0.3)) * (1 + 0.9 * exp(-z * h * 0.1)) * (1 - sstep(0.45, 1.65, z));
          if (e < 0.003) continue;
          const tl = clamp(z / 0.7, 0, 1);
          const fr = exp(-Math.max(0, z) * h * 0.3) * 0.55;
          const tp = sstep(0.35, 1, z);
          let r = lerp(lerp(lerp(C.lo[0], C.hi[0], tl), C.top[0], tp), C.fr[0], fr * 0.5);
          let g = lerp(lerp(lerp(C.lo[1], C.hi[1], tl), C.top[1], tp), C.fr[1], fr * 0.5);
          let b = lerp(lerp(lerp(C.lo[2], C.hi[2], tl), C.top[2], tp), C.fr[2], fr * 0.5);
          const w = e * I;
          r *= w;
          g *= w;
          b *= w;
          const i0 = y * W + xi;
          if (xi >= 0 && xi < W) {
            Er[i0] += r * (1 - fx);
            Eg[i0] += g * (1 - fx);
            Eb[i0] += b * (1 - fx);
          }
          if (xi + 1 < W && xi + 1 >= 0) {
            Er[i0 + 1] += r * fx;
            Eg[i0 + 1] += g * fx;
            Eb[i0 + 1] += b * fx;
          }
        }
      }
    }
    const sm = Math.max(1, round(W / 700));
    const gl = round(W / 40);
    const Ar = blur(Er, W, H, sm);
    const Ag = blur(Eg, W, H, sm);
    const Ab = blur(Eb, W, H, sm);
    const Gr = blur(blur(Er, W, H, gl), W, H, gl);
    const Gg = blur(blur(Eg, W, H, gl), W, H, gl);
    const Gb = blur(blur(Eb, W, H, gl), W, H, gl);
    const st = new Float64Array(N);
    const ns = round((num(p, "stars") * N) / 700);
    for (let s = 0; s < ns; s++) {
      const x = Math.floor(R() * W);
      const y = Math.floor(R() * H);
      const b = pow(R(), 4.5) * 1.6 + 0.05;
      st[y * W + x] += b * (1 - sstep(0.6, 1, y / H) * 0.8);
    }
    const stB = blur(st, W, H, 1);
    const d = newImage(W, H);
    for (let y = 0; y < H; y++) {
      const v = y / H;
      for (let x = 0; x < W; x++) {
        const i = y * W + x;
        let r = 0.003 + 0.006 * v * v * v;
        let g = 0.005 + 0.01 * pow(v, 2.6);
        let b = 0.014 + 0.022 * pow(v, 2.2);
        const s = st[i] * 0.85 + stB[i] * 0.6;
        r += s + Ar[i] + Gr[i] * 0.35;
        g += s + Ag[i] + Gg[i] * 0.35;
        b += s * 1.05 + Ab[i] + Gb[i] * 0.35;
        const n = (hash2(x, y, S) - 0.5) * 0.004;
        put(d, i, enc(1 - exp(-(r + n) * 1.4)), enc(1 - exp(-(g + n) * 1.4)), enc(1 - exp(-(b + n) * 1.4)));
      }
    }
    return d;
  },
};
