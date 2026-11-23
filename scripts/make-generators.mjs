// Renders docs/generators.png: one tile per generator, 4 × 3, from the compiled package.
//   cd packages/swirlseed && npm run build && cd ../.. && node scripts/make-generators.mjs
import { writeFileSync } from "node:fs";
import { deflateSync } from "node:zlib";
import { renderGenerator } from "../packages/swirlseed/dist/index.js";

const TILES = [
  ["sky", {}],
  ["ebru", { palette: "klasik" }],
  ["suminagashi", {}],
  ["mesh-gradient", {}],
  ["flow-field", { style: "gold" }],
  ["watercolor", {}],
  ["wet-on-wet", {}],
  ["aurora", {}],
  ["planets", { type: "ringed-giant" }],
  ["dendritic-painting", {}],
  ["paste-paper", {}],
  ["metaballs", { look: "glass" }],
];
const W = 300;
const H = 200;
const COLS = 4;
const ROWS = Math.ceil(TILES.length / COLS);
const OW = W * COLS;
const OH = H * ROWS;
const raw = Buffer.alloc(OH * (OW * 3 + 1));
TILES.forEach(([id, p], k) => {
  const px = renderGenerator(id, "Hello, world!", W, H, p);
  const ox = (k % COLS) * W;
  const oy = Math.floor(k / COLS) * H;
  for (let y = 0; y < H; y++)
    for (let x = 0; x < W; x++) {
      const s = (y * W + x) * 4;
      const d = (oy + y) * (OW * 3 + 1) + 1 + (ox + x) * 3;
      raw[d] = px[s];
      raw[d + 1] = px[s + 1];
      raw[d + 2] = px[s + 2];
    }
});
const table = new Uint32Array(256).map((_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
const crc = (b) => {
  let c = 0xffffffff;
  for (const x of b) c = table[(c ^ x) & 255] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
};
const chunk = (type, data) => {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const td = Buffer.concat([Buffer.from(type), data]);
  const c = Buffer.alloc(4);
  c.writeUInt32BE(crc(td));
  return Buffer.concat([len, td, c]);
};
const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(OW, 0);
ihdr.writeUInt32BE(OH, 4);
ihdr[8] = 8;
ihdr[9] = 2;
const png = Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk("IHDR", ihdr), chunk("IDAT", deflateSync(raw, { level: 9 })), chunk("IEND", Buffer.alloc(0))]);
writeFileSync(new URL("../docs/generators.png", import.meta.url), png);
console.log(`docs/generators.png ${OW}x${OH}`);
