/**
 * Deterministic math.
 *
 * `Math.sin`, `Math.exp`, `Math.pow`, … are allowed to differ in the last bits between engines
 * (V8, SpiderMonkey, JavaScriptCore) and from libm in Rust. To get byte-identical output in
 * every runtime, swirlseed uses only these functions, which are built from `+ - * /`, `floor`
 * and `sqrt` (all exactly specified by IEEE 754). The Rust crate contains the same code in
 * `dmath.rs`; keep the two files in sync, expression by expression.
 */

export const PI = 3.141592653589793;
export const TAU = 6.283185307179586;
const HALF_PI = 1.5707963267948966;
const LN2 = 0.6931471805599453;

/** 1/n! for n = 0..32, computed identically in every language. */
const INV_FACT: number[] = (() => {
  const out = [1];
  let f = 1;
  for (let i = 1; i <= 32; i++) {
    f *= i;
    out.push(1 / f);
  }
  return out;
})();

const SIN_C: number[] = Array.from({ length: 12 }, (_, k) => (k % 2 === 1 ? -INV_FACT[2 * k + 1] : INV_FACT[2 * k + 1]));
const COS_C: number[] = Array.from({ length: 13 }, (_, k) => (k % 2 === 1 ? -INV_FACT[2 * k] : INV_FACT[2 * k]));
const EXP_C: number[] = INV_FACT.slice(0, 16);
const LN_C: number[] = Array.from({ length: 13 }, (_, k) => 1 / (2 * k + 1));

function horner(c: number[], x: number): number {
  let p = c[c.length - 1];
  for (let i = c.length - 2; i >= 0; i--) p = p * x + c[i];
  return p;
}

/** x reduced to [-π, π). */
function reduce(x: number): number {
  const k = Math.floor(x / TAU + 0.5);
  return x - k * TAU;
}

export function sin(x: number): number {
  let r = reduce(x);
  if (r > HALF_PI) r = PI - r;
  else if (r < -HALF_PI) r = -PI - r;
  return r * horner(SIN_C, r * r);
}

export function cos(x: number): number {
  let r = Math.abs(reduce(x));
  let s = 1;
  if (r > HALF_PI) {
    r = PI - r;
    s = -1;
  }
  return s * horner(COS_C, r * r);
}

/** Exact 2^k for integer k (binary exponentiation of an exact power of two). */
function pow2i(k: number): number {
  let n = Math.abs(k) | 0;
  let base = k < 0 ? 0.5 : 2;
  let r = 1;
  while (n > 0) {
    if ((n & 1) === 1) r *= base;
    base *= base;
    n >>= 1;
  }
  return r;
}

export function exp(x: number): number {
  if (x < -745) return 0;
  if (x > 709) return Infinity;
  const k = Math.floor(x / LN2 + 0.5);
  const r = x - k * LN2;
  return horner(EXP_C, r) * pow2i(k);
}

/** Natural logarithm for finite x > 0. */
export function ln(x: number): number {
  let e = 0;
  while (x >= 1.5) {
    x *= 0.5;
    e += 1;
  }
  while (x < 0.75) {
    x *= 2;
    e -= 1;
  }
  const s = (x - 1) / (x + 1);
  return 2 * s * horner(LN_C, s * s) + e * LN2;
}

/** x^y for x ≥ 0 (0 for x ≤ 0). */
export function pow(x: number, y: number): number {
  if (x <= 0) return 0;
  return exp(y * ln(x));
}

/** Round half up (like `Math.round`, but spelled out so Rust can mirror it exactly). */
export function roundHalfUp(x: number): number {
  return Math.floor(x + 0.5);
}

const ATAN_C: number[] = Array.from({ length: 14 }, (_, k) => (k % 2 === 1 ? -1 : 1) / (2 * k + 1));

/** Arctangent, built from the series after two half-angle reductions (|x| ≤ tan(π/16)). */
export function atan(x: number): number {
  let s = 1;
  if (x < 0) {
    x = -x;
    s = -1;
  }
  let off = 0;
  if (x > 1) {
    x = 1 / x;
    off = 1;
  }
  x = x / (1 + Math.sqrt(1 + x * x));
  x = x / (1 + Math.sqrt(1 + x * x));
  let r = 4 * x * horner(ATAN_C, x * x);
  if (off === 1) r = HALF_PI - r;
  return s * r;
}

/** atan2(y, x) in (-π, π]. */
export function atan2(y: number, x: number): number {
  if (x > 0) return atan(y / x);
  if (x < 0) return y >= 0 ? atan(y / x) + PI : atan(y / x) - PI;
  if (y > 0) return HALF_PI;
  if (y < 0) return -HALF_PI;
  return 0;
}

export function asin(x: number): number {
  x = x < -1 ? -1 : x > 1 ? 1 : x;
  return atan2(x, Math.sqrt(1 - x * x));
}

export function acos(x: number): number {
  x = x < -1 ? -1 : x > 1 ? 1 : x;
  return atan2(Math.sqrt(1 - x * x), x);
}

/** Real cube root (sign kept). */
export function cbrt(x: number): number {
  if (x === 0) return 0;
  return x < 0 ? -pow(-x, 1 / 3) : pow(x, 1 / 3);
}
