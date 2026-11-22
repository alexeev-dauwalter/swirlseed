import { cos, roundHalfUp, sin } from "./dmath.js";
import { type Lab, labToSrgb255 } from "./color.js";
import type { HalftoneOptions } from "./options.js";
import type { Stop } from "./params.js";

const clamp01 = (v: number): number => (v < 0 ? 0 : v > 1 ? 1 : v);

/** Halftone pass over a float RGB buffer (0…255 per channel). `scale` = min(W, H) / 512. */
export function applyHalftone(
  rgb: Float32Array,
  W: number,
  H: number,
  scale: number,
  h: HalftoneOptions,
  stops: Stop[],
): void {
  const cs = Math.max(2, (h.size ?? 10) * scale);
  const ang = h.angle ?? 0.5236;
  const c = cos(ang);
  const s = sin(ang);
  const shape = h.shape ?? "dot";
  const mono = (h.mode ?? "color") === "mono";
  const invert = h.invert ?? false;
  const mix = clamp01(h.mix ?? 1);

  // "paper" and "ink" come from the lightest and darkest palette stops
  const sorted: Lab[] = [...stops].sort((a, b) => a.L - b.L);
  const darkC = labToSrgb255(sorted[0]);
  const lightC = labToSrgb255(sorted[sorted.length - 1]);
  const paper = invert ? darkC : lightC;
  const inkMono = invert ? lightC : darkC;

  const src = rgb.slice();
  let sr = 0;
  let sg = 0;
  let sb = 0;
  let slum = 0;
  const sampleAt = (ix: number, iy: number): void => {
    const sx = Math.min(W - 1, Math.max(0, roundHalfUp(ix - 0.5)));
    const sy = Math.min(H - 1, Math.max(0, roundHalfUp(iy - 0.5)));
    const o3 = (sy * W + sx) * 3;
    sr = src[o3];
    sg = src[o3 + 1];
    sb = src[o3 + 2];
    slum = clamp01((0.2126 * sr + 0.7152 * sg + 0.0722 * sb) / 255);
  };

  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const px = x + 0.5;
      const py = y + 0.5;
      const gx = px * c + py * s;
      const gy = -px * s + py * c;

      let best = 0;
      let ir = 0;
      let ig = 0;
      let ib = 0;

      if (shape === "line") {
        const j0 = Math.floor(gy / cs - 0.5);
        for (let dj = 0; dj < 2; dj++) {
          const cyc = (j0 + dj + 0.5) * cs;
          sampleAt(gx * c - cyc * s, gx * s + cyc * c);
          const a = invert ? slum : 1 - slum;
          const half = 0.5 * cs * a;
          const cov = clamp01(half - Math.abs(gy - cyc) + 0.5);
          if (cov > best) {
            best = cov;
            ir = sr;
            ig = sg;
            ib = sb;
          }
        }
      } else {
        const i0 = Math.floor(gx / cs - 0.5);
        const j0 = Math.floor(gy / cs - 0.5);
        for (let dj = 0; dj < 2; dj++) {
          for (let di = 0; di < 2; di++) {
            const cxc = (i0 + di + 0.5) * cs;
            const cyc = (j0 + dj + 0.5) * cs;
            sampleAt(cxc * c - cyc * s, cxc * s + cyc * c);
            const a = invert ? slum : 1 - slum;
            const sq = Math.sqrt(a);
            let cov: number;
            if (shape === "square") {
              const d = Math.max(Math.abs(gx - cxc), Math.abs(gy - cyc));
              cov = clamp01(0.5 * cs * sq - d + 0.5);
            } else {
              const ddx = gx - cxc;
              const ddy = gy - cyc;
              const d = Math.sqrt(ddx * ddx + ddy * ddy);
              cov = clamp01(0.7071 * cs * sq - d + 0.5);
            }
            if (cov > best) {
              best = cov;
              ir = sr;
              ig = sg;
              ib = sb;
            }
          }
        }
      }

      const inkR = mono ? inkMono[0] : ir;
      const inkG = mono ? inkMono[1] : ig;
      const inkB = mono ? inkMono[2] : ib;
      const o3 = (y * W + x) * 3;
      const outR = paper[0] + (inkR - paper[0]) * best;
      const outG = paper[1] + (inkG - paper[1]) * best;
      const outB = paper[2] + (inkB - paper[2]) * best;
      rgb[o3] = src[o3] + (outR - src[o3]) * mix;
      rgb[o3 + 1] = src[o3 + 1] + (outG - src[o3 + 1]) * mix;
      rgb[o3 + 2] = src[o3 + 2] + (outB - src[o3 + 2]) * mix;
    }
  }
}
