// Golden tests for the generators. The fixture crates/swirlseed/testdata/generators.txt holds both
// the cases and their SHA-256, so the Rust test (crates/swirlseed/tests/generators.rs) reads the same
// list and cannot drift from it.
//
//   npm test                 verify against the fixture
//   npm run golden           regenerate the fixture (only when an algorithm changes on purpose)

import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { test } from "node:test";

import { GENERATORS, getGenerator, renderGenerator, resolveParams } from "../dist/index.js";
import * as dm from "../dist/dmath.js";

const FIXTURE = new URL("../../../crates/swirlseed/testdata/generators.txt", import.meta.url);
const sha = (bytes) => createHash("sha256").update(bytes).digest("hex");

/** name, generator, seed, width, height, params. */
const CASES = [
  ["sky-default", "sky", "Hello, world!", 96, 60, {}],
  ["sky-noon-strato", "sky", "noon", 96, 60, { time: 12.5, type: "stratocumulus", cover: 0.7 }],
  ["sky-dawn-cirrus", "sky", "dawn", 96, 60, { time: 6.6, type: "cirrus", cover: 0.6 }],
  ["sky-night-moon", "sky", "night", 96, 60, { time: 23, moon: 0.5, cover: 0.2 }],
  ["sky-dusk-stratus", "sky", "dusk", 96, 60, { time: 19.2, type: "stratus", cover: 1, haze: 0.8 }],
  ["ebru-default", "ebru", "Hello, world!", 96, 64, {}],
  ["ebru-battal", "ebru", "Пётр", 96, 64, { style: "battal", palette: "deniz" }],
  ["ebru-gel-git", "ebru", "Autelon", 96, 64, { style: "gel-git", palette: "klasik", density: 1.4 }],
  ["ebru-bulbul", "ebru", "Kinsward", 96, 64, { style: "bulbul-yuvasi", palette: "gul" }],
  ["ebru-hatip", "ebru", "Granite", 96, 64, { style: "hatip", palette: "toprak", size: 1.5 }],
  ["sumi-default", "suminagashi", "Hello, world!", 96, 64, {}],
  ["sumi-ai", "suminagashi", "ai", 96, 64, { ink: "ai", rings: 50, breath: 0 }],
  ["sumi-mixed", "suminagashi", "mixed", 96, 64, { ink: "sumi-ai", centers: 3 }],
  ["sumi-beni", "suminagashi", "beni", 96, 64, { ink: "beni", breath: 1 }],
  ["mesh-default", "mesh-gradient", "Hello, world!", 96, 64, {}],
  ["mesh-random", "mesh-gradient", "random", 96, 64, { palette: "random", folds: 5 }],
  ["mesh-sky", "mesh-gradient", "sky", 64, 96, { palette: "sky", folds: 0, grain: 0 }],
  ["mesh-neon", "mesh-gradient", "neon", 96, 64, { palette: "neon", swirl: 1, flow: 1 }],
  ["flow-default", "flow-field", "Hello, world!", 96, 64, {}],
  ["flow-neon", "flow-field", "neon", 96, 64, { style: "neon" }],
  ["flow-gold", "flow-field", "gold", 96, 64, { style: "gold", swirl: 3 }],
  ["flow-sunset", "flow-field", "sunset", 96, 64, { style: "sunset", spacing: 0.02, length: 0.2 }],
  ["watercolor-default", "watercolor", "Hello, world!", 96, 64, {}],
  ["watercolor-sepia", "watercolor", "sepia", 96, 64, { palette: "sepia", washes: 3, bleed: 1, splatter: 1 }],
  ["wet-default", "wet-on-wet", "Hello, world!", 96, 64, {}],
  ["wet-ocean", "wet-on-wet", "ocean", 96, 64, { palette: "ocean", water: 1, bloom: 1 }],
  ["aurora-default", "aurora", "Hello, world!", 96, 60, {}],
  ["aurora-mixed", "aurora", "mixed", 96, 60, { colour: "mixed", curtains: 3, intensity: 2 }],
  ["aurora-violet", "aurora", "violet", 96, 60, { colour: "violet", curtains: 1, stars: 0 }],
  ["planets-rocky", "planets", "Hello, world!", 96, 64, {}],
  ["planets-ice", "planets", "ice", 96, 64, { type: "ice", sun: 40 }],
  ["planets-gas", "planets", "gas", 96, 64, { type: "gas-giant", sun: 0 }],
  ["planets-ringed", "planets", "ringed", 96, 64, { type: "ringed-giant", sun: -150 }],
  ["planets-nebula", "planets", "nebula", 160, 100, { type: "nebula" }],
  ["dendritic-default", "dendritic-painting", "Hello, world!", 96, 64, {}],
  ["dendritic-centre", "dendritic-painting", "centre", 96, 64, { origin: "centre", palette: "ember" }],
  ["dendritic-scatter", "dendritic-painting", "scatter", 96, 64, { origin: "scatter", palette: "spectrum", weight: 2 }],
  ["paste-waves", "paste-paper", "Hello, world!", 96, 64, {}],
  ["paste-basket", "paste-paper", "basket", 96, 64, { pattern: "basket", colour: "terracotta" }],
  ["paste-rings", "paste-paper", "rings", 96, 64, { pattern: "rings", colour: "plum" }],
  ["paste-stamped", "paste-paper", "stamped", 96, 64, { pattern: "stamped", colour: "ochre" }],
  ["metaballs-chrome", "metaballs", "Hello, world!", 96, 64, {}],
  ["metaballs-glass", "metaballs", "glass", 96, 64, { look: "glass" }],
  ["metaballs-glossy", "metaballs", "glossy", 96, 64, { look: "glossy", balls: 16 }],
  ["metaballs-lava", "metaballs", "lava", 96, 64, { look: "lava" }],
  ["metaballs-iridescent", "metaballs", "iridescent", 96, 64, { look: "iridescent", light: 40 }],
  ["metaballs-layers", "metaballs", "layers", 96, 64, { look: "layers" }],
  ["metaballs-gradient", "metaballs", "gradient", 96, 64, { look: "gradient" }],
  ["metaballs-drops", "metaballs", "drops", 96, 64, { look: "drops", balls: 18 }],
  ["metaballs-contour", "metaballs", "contour", 96, 64, { look: "contour", merge: 1 }],
  ["metaballs-holo", "metaballs", "holo", 96, 64, { look: "holo", size: 0.8 }],
];

const fmtParams = (p) =>
  Object.entries(p)
    .map(([k, v]) => `${k}=${v}`)
    .join(",") || "-";

/** Digest of the inverse-trigonometry and cube-root functions the generators use. */
function dmathDigest() {
  const buf = new DataView(new ArrayBuffer(2000 * 5 * 8));
  let off = 0;
  const put = (v) => {
    buf.setFloat64(off, v);
    off += 8;
  };
  for (let i = 0; i < 2000; i++) {
    const x = (i - 1000) * 0.0213;
    const u = (i - 1000) / 1000;
    put(dm.atan(x));
    put(dm.atan2(x, (i % 13) - 6));
    put(dm.asin(u));
    put(dm.acos(u));
    put(dm.cbrt(x * 7.3));
  }
  return sha(new Uint8Array(buf.buffer));
}

function computeFixture() {
  const lines = CASES.map(([name, id, seed, w, h, p]) => [name, id, seed, w, h, fmtParams(p), sha(renderGenerator(id, seed, w, h, p))].join("\t"));
  lines.push(["dmath-inverse", "-", "-", 0, 0, "-", dmathDigest()].join("\t"));
  return lines.join("\n") + "\n";
}

test("generator hashes match testdata/generators.txt (shared with Rust)", () => {
  const actual = computeFixture();
  if (process.env.UPDATE_GOLDEN === "1") {
    writeFileSync(FIXTURE, actual);
    console.log("generators.txt updated");
    return;
  }
  assert.equal(actual, readFileSync(FIXTURE, "utf8"));
});

test("every generator has a case and renders at its defaults", () => {
  for (const g of GENERATORS) {
    assert.ok(CASES.some((c) => c[1] === g.id), `no golden case for ${g.id}`);
    const px = renderGenerator(g.id, "x", 24, 16);
    assert.equal(px.length, 24 * 16 * 4);
    for (const q of g.params) {
      if (q.kind === "choice") assert.ok(q.options.some((o) => o.value === q.default), `${g.id}.${q.key} default`);
      else assert.ok(q.default >= q.min && q.default <= q.max, `${g.id}.${q.key} default in range`);
    }
  }
});

test("inverse trigonometry is accurate", () => {
  for (let i = 0; i <= 4000; i++) {
    const x = (i - 2000) * 0.0173;
    const u = (i - 2000) / 2000;
    assert.ok(Math.abs(dm.atan(x) - Math.atan(x)) < 1e-12, `atan ${x}`);
    assert.ok(Math.abs(dm.atan2(x, u) - Math.atan2(x, u)) < 1e-12, `atan2 ${x} ${u}`);
    assert.ok(Math.abs(dm.asin(u) - Math.asin(u)) < 1e-11, `asin ${u}`);
    assert.ok(Math.abs(dm.acos(u) - Math.acos(u)) < 1e-11, `acos ${u}`);
    assert.ok(Math.abs(dm.cbrt(x) - Math.cbrt(x)) < 1e-12 * Math.max(1, Math.abs(x)), `cbrt ${x}`);
  }
});

test("params: defaults, case-insensitive choices, errors", () => {
  const g = getGenerator("Metaballs");
  const p = resolveParams(g, { look: "GLASS", balls: 4 });
  assert.equal(p.look, "glass");
  assert.equal(p.balls, 4);
  assert.equal(p.merge, 0.55);
  assert.throws(() => resolveParams(g, { nope: 1 }), /unknown parameter/);
  assert.throws(() => resolveParams(g, { look: "plastic" }), /look/);
  assert.throws(() => getGenerator("nope"), /unknown generator/);
});

test("seed changes the picture", () => {
  for (const g of GENERATORS) {
    const a = renderGenerator(g.id, "alpha", 48, 32);
    const b = renderGenerator(g.id, "beta", 48, 32);
    assert.notDeepEqual(a, b, g.id);
    assert.deepEqual(a, renderGenerator(g.id, "alpha", 48, 32), g.id);
  }
});
