// Golden tests. `testdata/golden.txt` is shared with the Rust crate (crates/swirlseed/tests/parity.rs):
// both implementations must produce exactly the same SHA-256 for every case.
//
//   npm test                 verify against testdata/golden.txt
//   npm run golden           regenerate testdata/golden.txt (only when the algorithm changes on purpose)

import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { test } from "node:test";

import { createPattern, generate, paletteHex } from "../dist/index.js";
import * as dm from "../dist/dmath.js";

const GOLDEN = new URL("../../../testdata/golden.txt", import.meta.url);
const sha = (bytes) => createHash("sha256").update(bytes).digest("hex");

/** Keep this list identical to `cases()` in crates/swirlseed/tests/parity.rs. */
const CASES = [
  ["base-hello", "Hello, world!", 128, 128, {}],
  ["base-nonsquare", "Hello, world", 160, 96, {}],
  ["no-shine-cyrillic", "Пётр", 128, 128, { shine: 0 }],
  ["shine-strong", "alxv.life", 128, 128, { shine: 1.2 }],
  ["ribs-explicit", "Autelon", 144, 144, { ribs: { count: 18, strength: 0.7, angle: 0.3, shade: 0.5 } }],
  ["ribs-default", "glass", 128, 128, { ribs: {} }],
  ["halftone-default", "Granite", 128, 128, { halftone: {} }],
  [
    "halftone-square-mono-invert",
    "Kinsward",
    130,
    130,
    { halftone: { size: 9, angle: 0.2, shape: "square", mode: "mono", invert: true, mix: 0.8 } },
  ],
  ["halftone-line", "42", 140, 140, { halftone: { shape: "line", mode: "mono", size: 8, angle: 0.3 } }],
  ["dither-explicit", "seed", 128, 128, { dither: { levels: 4, scale: 1.5, amount: 1 } }],
  ["dither-default", "Hermes", 128, 128, { dither: {} }],
  ["grain", "Narra", 128, 128, { grain: { amount: 0.5, size: 2 } }],
  [
    "all-effects",
    "neural networks",
    200,
    120,
    { ribs: { count: 20 }, halftone: { size: 7, mix: 0.5 }, grain: {}, dither: { levels: 8 } },
  ],
  ["empty-seed", "", 64, 64, {}],
  ["size-512", "Hello, world!", 512, 512, {}],
];

function dmathDigest() {
  const buf = new DataView(new ArrayBuffer(2000 * 5 * 8));
  let off = 0;
  const put = (v) => {
    buf.setFloat64(off, v);
    off += 8;
  };
  for (let i = 0; i < 2000; i++) {
    const x = (i - 1000) * 0.0137;
    put(dm.sin(x));
    put(dm.cos(x));
    put(dm.exp((i - 1000) * 0.04));
    put(dm.ln(0.001 + i * 0.01));
    put(dm.pow(i / 2000, 0.8 + (i % 7) * 0.1));
  }
  return sha(new Uint8Array(buf.buffer));
}

function moodsDigest() {
  let s = "";
  for (let i = 0; i < 40; i++) s += sha(generate(`seed${i}`, 48, 48, {}));
  return sha(Buffer.from(s, "utf8"));
}

function computeGolden() {
  const lines = CASES.map(([name, seed, w, h, opts]) => `${name}\t${sha(generate(seed, w, h, opts))}`);
  lines.push(`dmath\t${dmathDigest()}`);
  lines.push(`moods\t${moodsDigest()}`);
  return lines.join("\n") + "\n";
}

test("golden hashes match testdata/golden.txt (shared with Rust)", () => {
  const actual = computeGolden();
  if (process.env.UPDATE_GOLDEN === "1") {
    writeFileSync(GOLDEN, actual);
    console.log("golden.txt updated");
    return;
  }
  assert.equal(actual, readFileSync(GOLDEN, "utf8"));
});

test("deterministic math is accurate", () => {
  for (let i = 0; i < 4000; i++) {
    const x = (i - 2000) * 0.0173;
    assert.ok(Math.abs(dm.sin(x) - Math.sin(x)) < 1e-12, `sin ${x}`);
    assert.ok(Math.abs(dm.cos(x) - Math.cos(x)) < 1e-12, `cos ${x}`);
    const e = (i - 2000) * 0.01;
    assert.ok(Math.abs(dm.exp(e) / Math.exp(e) - 1) < 1e-12, `exp ${e}`);
    const l = 1e-6 + i * 0.25;
    assert.ok(Math.abs(dm.ln(l) - Math.log(l)) < 1e-12, `ln ${l}`);
    const t = (i + 1) / 4001;
    assert.ok(Math.abs(dm.pow(t, 1.37) / Math.pow(t, 1.37) - 1) < 1e-11, `pow ${t}`);
  }
});

test("same seed, same pixels; one character changes the picture", () => {
  const a = generate("Hello, world!", 96, 96);
  assert.deepEqual(a, generate("Hello, world!", 96, 96));
  for (const other of ["Hello, world", "hello, world!", "Hello, world?"]) {
    const b = generate(other, 96, 96);
    let diff = 0;
    for (let i = 0; i < a.length; i += 4) diff += Math.abs(a[i] - b[i]) + Math.abs(a[i + 1] - b[i + 1]) + Math.abs(a[i + 2] - b[i + 2]);
    assert.ok(diff / (a.length / 4 * 3) > 20, `"${other}" looks too similar`);
  }
});

test("NFC: precomposed and combining characters give the same pattern", () => {
  assert.deepEqual(generate("café", 64, 64), generate("café", 64, 64));
});

test("look does not depend on frame size (mean brightness)", () => {
  const mean = (px) => {
    let s = 0;
    for (let i = 0; i < px.length; i += 4) s += px[i] + px[i + 1] + px[i + 2];
    return s / (px.length / 4 * 3);
  };
  const small = mean(generate("alxv.life", 128, 128));
  const large = mean(generate("alxv.life", 512, 512));
  assert.ok(Math.abs(small - large) < 3, `${small} vs ${large}`);
});

test("params are inspectable", () => {
  const p = createPattern("Hello, world!");
  assert.ok(["classic", "night", "pastel", "mono"].includes(p.mood));
  const hex = paletteHex(p);
  assert.ok(hex.length >= 4 && hex.every((h) => /^#[0-9a-f]{6}$/.test(h)));
});
