/** Planets and nebulae: per-pixel shaded spheres (3D noise terrain, clouds, rings) and domain-warped nebulae. */
import { DEG, atan2, clamp, cos, enc, exp, fbm2, fbm3, hypot3, lin3, newImage, oklab2lin, oklchFit, pow, put, round, sin, sstep, stream, vnoise, type V3 } from "./base.js";
import { Mask, capsule } from "./raster.js";
import { type Generator, num, opt, str } from "./spec.js";

const mix3 = (a: V3, b: V3, t: number): V3 => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
function ramp(s: V3[], t: number): V3 {
  t = clamp(t, 0, 1) * (s.length - 1);
  const i = Math.min(s.length - 2, Math.floor(t));
  return mix3(s[i], s[i + 1], t - i);
}
const GAS: V3[][] = [
  [[228, 196, 150], [190, 140, 96], [240, 222, 190], [158, 100, 70], [214, 170, 120]],
  [[210, 226, 236], [120, 158, 190], [232, 240, 246], [86, 124, 164], [170, 196, 214]],
  [[236, 190, 120], [176, 98, 60], [246, 226, 180], [120, 70, 52], [208, 150, 96]],
].map((g) => g.map((c) => lin3(c as V3)));
const C = (r: number, g: number, b: number): V3 => lin3([r, g, b]);
const OCEAN_S = C(70, 150, 186);
const OCEAN_D = C(8, 28, 78);
const FOREST = C(34, 78, 34);
const GRASS = C(88, 120, 52);
const DESERT = C(196, 160, 104);
const ROCK = C(120, 104, 92);
const SNOW = C(240, 240, 244);
const ICECAP = C(236, 242, 248);
const ICE_A = C(196, 226, 240);
const ICE_B = C(240, 248, 252);
const ICE_C = C(92, 148, 190);
const SPOT_A = C(190, 70, 50);
const SPOT_B = C(232, 170, 120);
const RING_A = C(150, 128, 104);
const RING_B = C(226, 206, 170);

export const planets: Generator = {
  id: "planets",
  title: "Planets & nebulae",
  description: "Worlds lit by a distant sun: oceans and clouds, cracked ice, banded and ringed giants, and glowing nebulae.",
  aspect: 1.5,
  params: [
    { kind: "choice", key: "type", label: "Type", default: "rocky", options: [opt("rocky", "Rocky"), opt("ice", "Ice"), opt("gas-giant", "Gas giant"), opt("ringed-giant", "Ringed giant"), opt("nebula", "Nebula")] },
    { kind: "number", key: "sun", label: "Sun", min: -170, max: 170, step: 1, default: -55, unit: "°" },
    { kind: "number", key: "size", label: "Size", min: 0.25, max: 0.5, step: 0.01, default: 0.38, unit: "%" },
    { kind: "number", key: "clouds", label: "Clouds", min: 0, max: 1, step: 0.01, default: 0.55, unit: "%" },
    { kind: "number", key: "water", label: "Water", min: 0, max: 1, step: 0.01, default: 0.5, unit: "%" },
  ],
  previews: [
    { title: "Type", sub: "One seed, five kinds of sky object.", key: "type", values: ["rocky", "ice", "gas-giant", "ringed-giant", "nebula"], labels: ["Rocky", "Ice", "Gas giant", "Ringed giant", "Nebula"] },
    { title: "Sun angle", sub: "The light moves around the planet, from crescent to full.", key: "sun", values: [-150, -70, 0, 70], labels: ["Behind", "Left", "Front", "Right"] },
  ],
  render(seed, W, H, p) {
    const R = stream(seed, "planets");
    const S = Math.floor(R() * 1e6);
    const T = str(p, "type");
    const neb = T === "nebula";
    const ringed = T === "ringed-giant";
    const giant = T === "gas-giant" || ringed;
    const clouds = num(p, "clouds");
    const m = Math.min(W, H);
    const sb = new Float64Array(W * H);
    const ns = round(((neb ? 1.1 : 0.6) * W * H) / 1500);
    const bright: number[] = [];
    for (let s = 0; s < ns; s++) {
      const x = Math.floor(R() * W);
      const y = Math.floor(R() * H);
      const b = pow(R(), 3.4);
      const hue = R();
      sb[y * W + x] = Math.max(sb[y * W + x], 0.2 + b * 0.8);
      if (b > 0.72 && neb) bright.push(x, y, b, hue);
    }
    const sunA = num(p, "sun") * DEG;
    let Lx = sin(sunA) * 0.92;
    let Ly = -0.28;
    let Lz = cos(sunA) * 0.92;
    {
      const l = hypot3(Lx, Ly, Lz);
      Lx /= l;
      Ly /= l;
      Lz /= l;
    }
    const r = (ringed ? 0.72 : 1) * num(p, "size") * H;
    const cx = W / 2;
    const cy = H / 2;
    const inv = 1 / r;
    const tilt = ringed ? 0.32 + R() * 0.3 : 0;
    const roll = (R() - 0.5) * 0.5;
    let npx = 0;
    let npy = cos(tilt);
    const npz = -sin(tilt);
    {
      const c = cos(roll);
      const s = sin(roll);
      const x = npx * c - npy * s;
      npy = npx * s + npy * c;
      npx = x;
    }
    const gas = GAS[Math.floor(R() * 3)];
    const fq = 2.6 + R() * 2.6;
    const sx = 0.3 + R() * 0.2;
    const sy0 = R() * 0.3;
    const sy = 0.18 + sy0 * (R() < 0.5 ? 1 : -1);
    R();
    const ox = R() * 50;
    const oy = R() * 50;
    const oz = R() * 50;
    const wl = 0.38 + num(p, "water") * 0.24;
    const hA = R() * 360;
    const hB = hA + 60 + R() * 90;
    const hC = hA + 180 + (R() - 0.5) * 60;
    const cA = oklchFit(0.5, 0.17, hA);
    const cB = oklchFit(0.62, 0.2, hB % 360);
    const cC = oklchFit(0.42, 0.13, hC % 360);
    const lA = oklab2lin(cA[0], cA[1], cA[2]);
    const lB = oklab2lin(cB[0], cB[1], cB[2]);
    const lC = oklab2lin(cC[0], cC[1], cC[2]);
    const ringAt = (rho: number): number => {
      if (rho < 1.28 || rho > 2.35) return 0;
      const t = (rho - 1.28) / 1.07;
      let o = 0.35 + 0.6 * vnoise(rho * 70, 0, S) * (0.5 + vnoise(rho * 13, 1, S + 2));
      o *= sstep(0, 0.05, t) * sstep(1, 0.93, t);
      o *= 1 - 0.92 * sstep(0.045, 0, Math.abs(rho - 1.87));
      o *= 1 - 0.8 * sstep(0.03, 0, Math.abs(rho - 2.02));
      return clamp(o, 0, 0.97);
    };
    const ringCol = (rho: number): V3 => mix3(RING_A, RING_B, clamp(0.5 + 0.5 * sin(rho * 37) + 0.25 * vnoise(rho * 50, 3, S), 0, 1));
    const hvx = Lx;
    const hvy = Ly;
    const hvz = Lz + 1;
    const hl = hypot3(hvx, hvy, hvz);
    const out = new Float64Array(W * H * 3);
    for (let y = 0; y < H; y++)
      for (let x = 0; x < W; x++) {
        const u = (x - cx) * inv;
        const v = (y - cy) * inv;
        let R_: number;
        let G_: number;
        let B_: number;
        if (neb) {
          const px = (x / m) * 2.1;
          const py = (y / m) * 2.1;
          const q = fbm2(px + ox, py + oy, S, 4);
          const w = fbm2(px + 4 * q + 1.7, py + 4 * q + 9.2, S + 7, 4);
          const f = fbm2(px + 3.2 * w + 8, py + 3.2 * w + 2, S + 13, 5);
          const dens = pow(clamp(f * 1.45 - 0.18, 0, 1), 1.9);
          const t2 = clamp(w * 1.5 - 0.2, 0, 1);
          const col = mix3(mix3(lA, lB, t2), lC, clamp((q - 0.45) * 3, 0, 1) * 0.7);
          const lane = 1 - 0.88 * sstep(0.5, 0.72, fbm2(px * 2.6 + 30, py * 2.6 + 11, S + 21, 4)) * sstep(0.1, 0.5, f);
          const cq = clamp(f * 1.9 - 0.9, 0, 1);
          const core = cq * cq * 1.5;
          R_ = (col[0] * dens * 1.25 + core * 0.3) * lane + 0.004;
          G_ = (col[1] * dens * 1.25 + core * 0.27) * lane + 0.005;
          B_ = (col[2] * dens * 1.25 + core * 0.25) * lane + 0.01;
        } else {
          const t = (y / H) * (y / H);
          const nb = fbm2((x / m) * 1.6 + ox, (y / m) * 1.6 + oy, S + 3, 3);
          R_ = 0.003 + 0.004 * t + 0.012 * nb * nb * (lA[0] + 0.2);
          G_ = 0.004 + 0.006 * t + 0.012 * nb * nb * (lA[1] + 0.2);
          B_ = 0.01 + 0.016 * t + 0.02 * nb * nb * (lA[2] + 0.2);
        }
        const st1 = sb[y * W + x];
        if (st1 > 0) {
          R_ += st1;
          G_ += st1 * 0.97;
          B_ += st1 * (neb ? 1 : 0.9);
        }
        const d2 = u * u + v * v;
        const rn = Math.sqrt(d2);
        if (!neb) {
          if (rn < 1 + 2 / r) {
            const nz = Math.sqrt(Math.max(0, 1 - d2));
            const nx = u;
            const ny = v;
            const cov = clamp((1 - rn) * r + 0.5, 0, 1);
            const ndl = nx * Lx + ny * Ly + nz * Lz;
            const lit = sstep(-0.12, 0.35, ndl) * 0.9 + 0.1 * clamp(ndl + 0.2, 0, 1);
            let col: V3;
            let spec = 0;
            let cl = 0;
            const X = nx + ox;
            const Y = ny + oy;
            const Z = nz + oz;
            if (T === "rocky") {
              const wq = fbm3(X * 1.2 + 7, Y * 1.2, Z * 1.2, S + 40, 3) * 0.9;
              const h = fbm3(X * 1.9 + wq, Y * 1.9, Z * 1.9 + wq, S, 7) * 0.82 + 0.18 * fbm3(X * 9, Y * 9, Z * 9, S + 9, 4);
              const lat = Math.abs(ny);
              if (h < wl) {
                col = mix3(OCEAN_S, OCEAN_D, pow(clamp((wl - h) / 0.28, 0, 1), 0.6));
                const hd = clamp((nx * hvx + ny * hvy + nz * hvz) / hl, 0, 1);
                spec = pow(hd, 260) * 0.7 + pow(hd, 24) * 0.06;
              } else {
                const k = clamp((h - wl) / 0.3, 0, 1);
                const dry = clamp(fbm3(X * 2.4 + 5, Y * 2.4, Z * 2.4, S + 4, 3) * 1.5 - 0.3 + lat * 0.2, 0, 1);
                col = mix3(mix3(mix3(FOREST, GRASS, clamp(dry * 2, 0, 1)), DESERT, clamp(dry * 2 - 1, 0, 1)), ROCK, k * k);
                if (k > 0.8) col = mix3(col, SNOW, (k - 0.8) * 4);
              }
              col = mix3(col, ICECAP, sstep(0.8, 0.9, lat + (fbm3(X * 4, Y * 4, Z * 4, S + 6, 3) - 0.5) * 0.14));
              const wx = fbm3(X * 1.6 + 3, Y * 1.6, Z * 1.6, S + 11, 3) * 2.2;
              const wy = fbm3(X * 1.6, Y * 1.6 + 5, Z * 1.6, S + 13, 3) * 2.2;
              const cn = fbm3(X * 2.4 + wx, Y * 3.2 + wy, Z * 2.4 + wx, S + 12, 7);
              const fine = fbm3(X * 14, Y * 14, Z * 14, S + 14, 3);
              cl = clamp(sstep(0.63 - 0.14 * clouds, 0.74 - 0.12 * clouds, cn * (0.85 + 0.3 * fine)) * (0.75 + 0.25 * fine) * Math.min(1, clouds * 2), 0, 0.97);
            } else if (T === "ice") {
              const h = fbm3(X * 2.2, Y * 2.2, Z * 2.2, S, 5);
              const cr = Math.abs(fbm3(X * 3.1 + 9, Y * 3.1, Z * 3.1, S + 5, 4) - 0.5);
              col = mix3(mix3(ICE_A, ICE_B, h), ICE_C, (1 - sstep(0, 0.035, cr)) * 0.8 + sstep(0.5, 0, h) * 0.15);
              spec = pow(clamp(nz, 0, 1), 12) * 0.12;
              cl = clouds * 0.3 * clamp(fbm3(X * 3, Y * 3, Z * 3, S + 12, 4) * 2 - 0.7, 0, 1);
            } else {
              const w = fbm3(X * 2.2, Y * 2.2, Z * 2.2, S + 3, 4);
              const lat = ny * fq;
              const shear = sin(lat * 3.1) * 1.3;
              const ed = fbm3(X * 5 + shear, Y * 14, Z * 5 - shear, S + 15, 5);
              const b = lat + w * 0.55 + (ed - 0.5) * 0.55 * Math.abs(cos(lat * 3.1)) + fbm3(X * 9, Y * 9 * 0.2, Z * 9, S + 8, 3) * 0.15;
              col = ramp(gas, 0.5 + 0.42 * sin(b * 3.1) + 0.18 * sin(b * 7.3 + 1) + 0.25 * (w - 0.5));
              const ex = (nx - sx) / 0.34;
              const ey = (ny - sy) / 0.17;
              const e2 = ex * ex + ey * ey;
              if (e2 < 1.6) {
                const sw = atan2(ey, ex) + e2 * 3;
                col = mix3(col, mix3(SPOT_A, SPOT_B, 0.5 + 0.5 * sin(sw * 2)), sstep(1.6, 0.4, e2) * 0.85);
              }
            }
            let shade = 1;
            if (ringed) {
              const dn = npx * Lx + npy * Ly + npz * Lz;
              if (Math.abs(dn) > 1e-4) {
                const t = -(npx * nx + npy * ny + npz * nz) / dn;
                if (t > 0) shade = 1 - ringAt(hypot3(nx + t * Lx, ny + t * Ly, nz + t * Lz)) * 0.85;
              }
            }
            const dl = lit * shade;
            let pr = col[0] * dl;
            let pg = col[1] * dl;
            let pb = col[2] * dl;
            if (cl > 0) {
              const cd = dl * 0.95 + 0.03;
              pr = pr * (1 - cl) + cd * cl;
              pg = pg * (1 - cl) + cd * cl;
              pb = pb * (1 - cl) + cd * cl;
            }
            pr += spec * lit;
            pg += spec * lit;
            pb += spec * lit;
            const rim = pow(1 - nz, 3.2) * clamp(ndl * 1.6 + 0.35, 0, 1);
            const ac: V3 = giant ? [0.5, 0.4, 0.3] : T === "ice" ? [0.35, 0.65, 1] : [0.22, 0.5, 1];
            const am = giant ? 0.25 : 0.9;
            pr += ac[0] * rim * am;
            pg += ac[1] * rim * am;
            pb += ac[2] * rim * am;
            R_ = R_ * (1 - cov) + pr * cov;
            G_ = G_ * (1 - cov) + pg * cov;
            B_ = B_ * (1 - cov) + pb * cov;
          }
          if (rn > 1) {
            const gl = exp(-(rn - 1) * 16) * clamp((u * Lx + v * Ly) * 0.9 + 0.5, 0, 1) * (giant ? 0.18 : 0.55);
            const ac: V3 = T === "ice" ? [0.3, 0.55, 1] : T === "rocky" ? [0.18, 0.42, 1] : [0.5, 0.4, 0.3];
            R_ += ac[0] * gl;
            G_ += ac[1] * gl;
            B_ += ac[2] * gl;
          }
          if (ringed) {
            const zz = -(npx * u + npy * v) / npz;
            const rho = hypot3(u, v, zz);
            const o = ringAt(rho);
            if (o > 0 && (rn >= 1 || zz > 0)) {
              let lt = 0.9;
              const bb = u * Lx + v * Ly + zz * Lz;
              if (bb < 0 && rho * rho - bb * bb < 1) lt = 0.08;
              const rc = ringCol(rho);
              R_ = R_ * (1 - o) + rc[0] * o * lt;
              G_ = G_ * (1 - o) + rc[1] * o * lt;
              B_ = B_ * (1 - o) + rc[2] * o * lt;
            }
          }
        }
        const i = (y * W + x) * 3;
        out[i] = enc(1 - exp(-R_));
        out[i + 1] = enc(1 - exp(-G_));
        out[i + 2] = enc(1 - exp(-B_));
      }
    if (neb) {
      // bright stars: soft additive glow and diffraction spikes
      const mask = new Mask(W, H);
      for (let k = 0; k < bright.length; k += 4) {
        const sx2 = bright[k];
        const sy2 = bright[k + 1];
        const b = bright[k + 2];
        const h = bright[k + 3];
        const rr = m * (0.008 + 0.02 * b);
        const col: V3 = h < 0.33 ? [255, 225, 190] : h < 0.66 ? [200, 220, 255] : [255, 255, 255];
        const x0 = Math.max(0, Math.floor(sx2 - rr));
        const x1 = Math.min(W - 1, Math.ceil(sx2 + rr));
        const y0 = Math.max(0, Math.floor(sy2 - rr));
        const y1 = Math.min(H - 1, Math.ceil(sy2 + rr));
        for (let y = y0; y <= y1; y++)
          for (let x = x0; x <= x1; x++) {
            const dx = x + 0.5 - sx2;
            const dy = y + 0.5 - sy2;
            const t = Math.sqrt(dx * dx + dy * dy) / rr;
            if (t >= 1) continue;
            const a = t < 0.25 ? 0.8 * b + (0.25 * b - 0.8 * b) * (t / 0.25) : 0.25 * b * (1 - (t - 0.25) / 0.75);
            const i = (y * W + x) * 3;
            out[i] += col[0] * a;
            out[i + 1] += col[1] * a;
            out[i + 2] += col[2] * a;
          }
        if (b > 0.85) {
          const L = rr * 2.2;
          const lw = Math.max(1, m * 0.0012);
          capsule(mask, sx2 - L, sy2, sx2 + L, sy2, lw);
          capsule(mask, sx2, sy2 - L, sx2, sy2 + L, lw);
          for (let y = mask.y0; y <= mask.y1; y++)
            for (let x = mask.x0; x <= mask.x1; x++) {
              const i = y * W + x;
              const c = Math.min(1, mask.cov[i]) * 0.5 * b;
              out[i * 3] += col[0] * c;
              out[i * 3 + 1] += col[1] * c;
              out[i * 3 + 2] += col[2] * c;
              mask.cov[i] = 0;
            }
          mask.x0 = 0;
          mask.y0 = 0;
          mask.x1 = -1;
          mask.y1 = -1;
        }
      }
    }
    const d = newImage(W, H);
    for (let i = 0; i < W * H; i++) put(d, i, out[i * 3], out[i * 3 + 1], out[i * 3 + 2]);
    return d;
  },
};
