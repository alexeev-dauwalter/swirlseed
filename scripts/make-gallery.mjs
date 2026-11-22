// Regenerates the images in docs/ from the built npm package:
//   cd packages/swirlseed && npm run build && cd ../.. && node scripts/make-gallery.mjs
import { writeFileSync, mkdirSync } from "node:fs";
import { deflateSync } from "node:zlib";
import { generate } from "../packages/swirlseed/dist/index.js";

const crcTable = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
const crc32 = (buf) => {
  let c = 0xffffffff;
  for (const b of buf) c = crcTable[(c ^ b) & 255] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
};
function chunk(type, data) {
  const out = Buffer.alloc(12 + data.length);
  out.writeUInt32BE(data.length, 0);
  out.write(type, 4, "ascii");
  Buffer.from(data).copy(out, 8);
  out.writeUInt32BE(crc32(out.subarray(4, 8 + data.length)), 8 + data.length);
  return out;
}
function png(rgb, w, h) {
  const raw = Buffer.alloc((w * 3 + 1) * h);
  for (let y = 0; y < h; y++) Buffer.from(rgb.buffer, rgb.byteOffset + y * w * 3, w * 3).copy(raw, y * (w * 3 + 1) + 1);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8;
  ihdr[9] = 2; // RGB
  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/** Lay tiles out in a grid with a small gap. */
function sheet(tiles, cols, size, gap = 10) {
  const rows = Math.ceil(tiles.length / cols);
  const w = cols * size + (cols + 1) * gap;
  const h = rows * size + (rows + 1) * gap;
  const rgb = new Uint8Array(w * h * 3).fill(244);
  tiles.forEach(([seed, opts], i) => {
    const rgba = generate(seed, size, size, opts);
    const ox = gap + (i % cols) * (size + gap);
    const oy = gap + Math.floor(i / cols) * (size + gap);
    for (let y = 0; y < size; y++)
      for (let x = 0; x < size; x++) {
        const s = (y * size + x) * 4;
        const d = ((oy + y) * w + ox + x) * 3;
        rgb[d] = rgba[s];
        rgb[d + 1] = rgba[s + 1];
        rgb[d + 2] = rgba[s + 2];
      }
  });
  return png(rgb, w, h);
}

mkdirSync(new URL("../docs/", import.meta.url), { recursive: true });
const out = (name, buf) => writeFileSync(new URL(`../docs/${name}`, import.meta.url), buf);

out("hello.png", sheet([["Hello, world!", {}], ["Hello, world", {}]], 2, 360));
out(
  "textures.png",
  sheet(
    [
      ["Hello, world!", {}],
      ["Hello, world!", { ribs: { count: 22 } }],
      ["Hello, world!", { halftone: { size: 9 } }],
      ["Hello, world!", { halftone: { shape: "line", mode: "mono", size: 8, angle: 0.3 } }],
      ["Hello, world!", { dither: { levels: 3, scale: 2.5 } }],
      ["Hello, world!", { ribs: { count: 30, strength: 0.8 }, grain: { amount: 0.3 }, halftone: { size: 7, mix: 0.3 } }],
    ],
    3,
    280,
  ),
);
out(
  "gallery.png",
  sheet(
    ["Hello, world!", "Autelon", "Granite", "Narra", "glass", "seed", "swirlseed", "alxv.life"].map((s) => [s, {}]),
    4,
    230,
  ),
);
console.log("docs/ updated");
