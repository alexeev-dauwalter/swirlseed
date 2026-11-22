/** cyrb128: fast 128-bit string hash with a strong avalanche effect (1 char changes every bit). */
export function cyrb128(str: string): [number, number, number, number] {
  let h1 = 1779033703,
    h2 = 3144134277,
    h3 = 1013904242,
    h4 = 2773480762;
  for (let i = 0; i < str.length; i++) {
    const k = str.charCodeAt(i);
    h1 = h2 ^ Math.imul(h1 ^ k, 597399067);
    h2 = h3 ^ Math.imul(h2 ^ k, 2869860233);
    h3 = h4 ^ Math.imul(h3 ^ k, 951274213);
    h4 = h1 ^ Math.imul(h4 ^ k, 2716044179);
  }
  h1 = Math.imul(h3 ^ (h1 >>> 18), 597399067);
  h2 = Math.imul(h4 ^ (h2 >>> 22), 2869860233);
  h3 = Math.imul(h1 ^ (h3 >>> 17), 951274213);
  h4 = Math.imul(h2 ^ (h4 >>> 19), 2716044179);
  h1 ^= h2 ^ h3 ^ h4;
  h2 ^= h1;
  h3 ^= h1;
  h4 ^= h1;
  return [h1 >>> 0, h2 >>> 0, h3 >>> 0, h4 >>> 0];
}

/** sfc32: small, fast PRNG with 128-bit state. Returns numbers in [0, 1). */
export class Rng {
  private a: number;
  private b: number;
  private c: number;
  private d: number;

  constructor(a: number, b: number, c: number, d: number) {
    this.a = a >>> 0;
    this.b = b >>> 0;
    this.c = c >>> 0;
    this.d = d >>> 0;
  }

  next(): number {
    let a = this.a >>> 0;
    let b = this.b >>> 0;
    let c = this.c >>> 0;
    let d = this.d >>> 0;
    let t = (a + b) | 0;
    a = b ^ (b >>> 9);
    b = (c + (c << 3)) | 0;
    c = (c << 21) | (c >>> 11);
    d = (d + 1) | 0;
    t = (t + d) | 0;
    c = (c + t) | 0;
    this.a = a;
    this.b = b;
    this.c = c;
    this.d = d;
    return (t >>> 0) / 4294967296;
  }

  /** a + (b - a) * next() */
  range(a: number, b: number): number {
    return a + (b - a) * this.next();
  }
}

/**
 * An independent random stream per aspect of the pattern ("geo", "pal", "noise").
 * The seed is NFC-normalised so that "é" typed as one or two code points gives the same pattern.
 */
export function makeRng(seed: string, stream: string): Rng {
  const [a, b, c, d] = cyrb128(seed.normalize("NFC") + "\u0000" + stream);
  const rng = new Rng(a, b, c, d);
  for (let i = 0; i < 15; i++) rng.next(); // warm-up
  return rng;
}
