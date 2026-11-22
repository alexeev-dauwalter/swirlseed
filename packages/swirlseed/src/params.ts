import { PI } from "./dmath.js";
import { type Lab, labToSrgb255, oklchFit } from "./color.js";
import { Rng, makeRng } from "./rng.js";

export interface WarpLayer {
  freq: number;
  amp: number;
  ox: number;
  oy: number;
  ox2: number;
  oy2: number;
}

export interface Stop extends Lab {
  pos: number;
}

export type Mood = "classic" | "night" | "pastel" | "mono";

/** Everything the seed decides. Inspectable; feed it to `renderPattern`. */
export interface PatternParams {
  seed: string;
  /** Noise permutation table (512 bytes, depends only on the seed). */
  perm: Uint8Array;
  /** Flow direction and how much the noise is stretched along it (long streaks). */
  angle: number;
  stretch: number;
  baseFreq: number;
  swirl: { cx: number; cy: number; strength: number; radius: number };
  layers: WarpLayer[];
  /** ramp — one pass through the palette; bands — several back-and-forth waves. */
  mode: "ramp" | "bands";
  repeats: number;
  phase: number;
  fbmWeight: number;
  gamma: number;
  /** Default light/gloss strength (override with `RenderOptions.shine`). */
  shine: number;
  mood: Mood;
  stops: Stop[];
}

/** Hue pairs (base, accent) in degrees: teal + red-orange, blue + red, and so on. */
const PAIRS: [number, number][] = [
  [215, 32],
  [255, 28],
  [235, 55],
  [305, 45],
  [180, 20],
  [275, 18],
  [20, 215],
  [160, 25],
  [330, 200],
];

function makePalette(rnd: Rng): { stops: Stop[]; mood: Mood } {
  const [baseA, baseB] = PAIRS[Math.floor(rnd.next() * PAIRS.length)];
  const hueA = baseA + rnd.range(-14, 14);
  const hueB = baseB + rnd.range(-14, 14);
  const hueC = hueB + rnd.range(14, 42);

  const r = rnd.next();
  const mood: Mood = r < 0.5 ? "classic" : r < 0.7 ? "night" : r < 0.86 ? "pastel" : "mono";

  let roles: Lab[];
  if (mood === "classic") {
    const dark = oklchFit(rnd.range(0.13, 0.24), rnd.range(0.02, 0.07), hueA);
    const deep = oklchFit(rnd.range(0.3, 0.46), rnd.range(0.07, 0.13), hueA + rnd.range(-15, 15));
    const accent = oklchFit(rnd.range(0.54, 0.64), rnd.range(0.15, 0.21), hueB);
    const warm = oklchFit(rnd.range(0.62, 0.72), rnd.range(0.1, 0.16), hueC);
    const light = oklchFit(rnd.range(0.9, 0.96), rnd.range(0.005, 0.03), hueA + rnd.range(-30, 30));
    const q = rnd.next();
    roles = q < 0.65 ? [dark, deep, accent, warm, light] : q < 0.85 ? [dark, deep, accent, warm] : [deep, accent, warm, light];
  } else if (mood === "night") {
    // near-black background and a "glowing" accent
    roles = [
      oklchFit(rnd.range(0.07, 0.13), rnd.range(0.01, 0.04), hueA),
      oklchFit(rnd.range(0.2, 0.3), rnd.range(0.06, 0.11), hueA),
      oklchFit(rnd.range(0.42, 0.52), rnd.range(0.14, 0.2), hueA + rnd.range(10, 30)),
      oklchFit(rnd.range(0.62, 0.72), rnd.range(0.17, 0.23), hueB),
      oklchFit(rnd.range(0.88, 0.95), rnd.range(0.03, 0.08), hueB + rnd.range(10, 30)),
    ];
  } else if (mood === "pastel") {
    roles = [
      oklchFit(rnd.range(0.58, 0.68), rnd.range(0.1, 0.14), hueA),
      oklchFit(rnd.range(0.74, 0.82), rnd.range(0.06, 0.1), hueB),
      oklchFit(rnd.range(0.93, 0.97), rnd.range(0.01, 0.03), hueA + rnd.range(-20, 20)),
      oklchFit(rnd.range(0.8, 0.88), rnd.range(0.05, 0.09), hueC),
      oklchFit(rnd.range(0.7, 0.78), rnd.range(0.07, 0.11), hueA + rnd.range(30, 60)),
    ];
  } else {
    // mono: one hue from dark to light plus a tiny accent of another hue
    const h = hueA;
    const peak = rnd.range(0.1, 0.17);
    roles = [
      oklchFit(rnd.range(0.12, 0.2), peak * 0.35, h),
      oklchFit(rnd.range(0.34, 0.42), peak * 0.75, h),
      oklchFit(rnd.range(0.54, 0.62), peak, h + rnd.range(-10, 10)),
      oklchFit(rnd.range(0.68, 0.76), peak * 0.9, hueB),
      oklchFit(rnd.range(0.9, 0.96), peak * 0.2, h),
    ];
  }

  const n = roles.length;
  const stops: Stop[] = roles.map((c, i) => {
    const jitter = i === 0 || i === n - 1 ? 0 : rnd.range(-0.07, 0.07);
    return { ...c, pos: Math.min(1, Math.max(0, i / (n - 1) + jitter)) };
  });
  if (rnd.next() < 0.5) {
    stops.reverse();
    for (const s of stops) s.pos = 1 - s.pos;
  }
  return { stops, mood };
}

/** Deterministically derive all pattern parameters from a seed string. */
export function createPattern(seed: string): PatternParams {
  // Independent streams: changing the palette code never moves the geometry, and vice versa.
  const noiseRng = makeRng(seed, "noise");
  const geo = makeRng(seed, "geo");
  const pal = makeRng(seed, "pal");

  const base = new Uint8Array(256);
  for (let i = 0; i < 256; i++) base[i] = i;
  for (let i = 255; i > 0; i--) {
    const j = Math.floor(noiseRng.next() * (i + 1));
    const t = base[i];
    base[i] = base[j];
    base[j] = t;
  }
  const perm = new Uint8Array(512);
  for (let i = 0; i < 512; i++) perm[i] = base[i & 255];

  const nLayers = 2 + Math.floor(geo.next() * 2); // 2–3
  const layers: WarpLayer[] = [];
  for (let i = 0; i < nLayers; i++) {
    layers.push({
      freq: geo.range(0.45, 0.85) * (1 + i * 0.4),
      amp: geo.range(0.75, 1.4) / (1 + i * 0.4),
      ox: geo.range(-50, 50),
      oy: geo.range(-50, 50),
      ox2: geo.range(-50, 50),
      oy2: geo.range(-50, 50),
    });
  }

  const sign = geo.next() < 0.5 ? -1 : 1;
  const mode = geo.next() < 0.55 ? "bands" : "ramp";
  const { stops, mood } = makePalette(pal);

  return {
    seed,
    perm,
    angle: geo.range(0, PI),
    stretch: geo.range(2.6, 6),
    baseFreq: geo.range(1.3, 2.1),
    swirl: {
      cx: geo.range(-0.35, 0.35),
      cy: geo.range(-0.3, 0.3),
      strength: sign * geo.range(0.5, 2.0),
      radius: geo.range(0.3, 0.6),
    },
    layers,
    mode,
    repeats: geo.range(0.8, 2.2),
    phase: geo.next(),
    fbmWeight: geo.range(0.15, 0.55),
    gamma: geo.range(0.8, 1.25),
    shine: geo.range(0.25, 0.6),
    mood,
    stops,
  };
}

/** The palette as `#rrggbb` strings, ordered by position along the ramp. */
export function paletteHex(p: PatternParams): string[] {
  return [...p.stops]
    .sort((a, b) => a.pos - b.pos)
    .map((s) => {
      const rgb = labToSrgb255(s);
      return (
        "#" +
        rgb
          .map((v) => Math.min(255, Math.max(0, Math.floor(v + 0.5))))
          .map((v) => v.toString(16).padStart(2, "0"))
          .join("")
      );
    });
}
