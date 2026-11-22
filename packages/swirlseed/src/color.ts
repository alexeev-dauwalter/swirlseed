import { PI, cos, pow, sin } from "./dmath.js";

export interface Lab {
  L: number;
  a: number;
  b: number;
}

export function oklabToLinear(L: number, a: number, b: number): [number, number, number] {
  const l_ = L + 0.3963377774 * a + 0.2158037573 * b;
  const m_ = L - 0.1055613458 * a - 0.0638541728 * b;
  const s_ = L - 0.0894841775 * a - 1.291485548 * b;
  const l = l_ * l_ * l_;
  const m = m_ * m_ * m_;
  const s = s_ * s_ * s_;
  return [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ];
}

export function linearToSrgb255(v: number): number {
  v = v < 0 ? 0 : v > 1 ? 1 : v;
  return 255 * (v <= 0.0031308 ? 12.92 * v : 1.055 * pow(v, 1 / 2.4) - 0.055);
}

/** OKLCH → OKLab, reducing chroma by bisection until the colour fits into sRGB. */
export function oklchFit(L: number, C: number, hDeg: number): Lab {
  const h = (hDeg * PI) / 180;
  const ch = cos(h);
  const sh = sin(h);
  const inGamut = (c: number): boolean => {
    const [r, g, b] = oklabToLinear(L, c * ch, c * sh);
    const e = 1e-4;
    return r >= -e && r <= 1 + e && g >= -e && g <= 1 + e && b >= -e && b <= 1 + e;
  };
  if (!inGamut(C)) {
    let lo = 0;
    let hi = C;
    for (let i = 0; i < 14; i++) {
      const mid = (lo + hi) / 2;
      if (inGamut(mid)) lo = mid;
      else hi = mid;
    }
    C = lo;
  }
  return { L, a: C * ch, b: C * sh };
}

/** OKLab → sRGB, each channel 0…255 (not rounded). */
export function labToSrgb255(l: Lab): [number, number, number] {
  const [r, g, b] = oklabToLinear(l.L, l.a, l.b);
  return [linearToSrgb255(r), linearToSrgb255(g), linearToSrgb255(b)];
}
