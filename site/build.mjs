// Assembles the static site into site/dist: the pages from site/src, one page per generator (built
// from src/gen.html and the generator's spec) and the compiled browser build of the library
// (packages/swirlseed/dist, plain ES modules). © 2026 Peter Alexeev Dauwalter
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const lib = join(here, "..", "packages", "swirlseed", "dist");
const out = join(here, "dist");

if (!existsSync(join(lib, "index.js")) || !existsSync(join(lib, "gen", "index.js"))) {
  console.error("packages/swirlseed/dist is missing. Run `npm install && npm run build` in packages/swirlseed first.");
  process.exit(1);
}
rmSync(out, { recursive: true, force: true });
cpSync(join(here, "src"), out, { recursive: true, filter: (p) => !p.endsWith("gen.html") });
cpSync(lib, join(out, "swirlseed"), { recursive: true, filter: (p) => !p.endsWith(".map") && !p.endsWith(".d.ts") });

const { GENERATORS } = await import(pathToFileURL(join(lib, "gen", "index.js")).href);
const tpl = readFileSync(join(here, "src", "gen.html"), "utf8");
const esc = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/"/g, "&quot;");
for (const g of GENERATORS) {
  mkdirSync(join(out, g.id), { recursive: true });
  const html = tpl.replaceAll("%ID%", g.id).replaceAll("%TITLE%", esc(g.title)).replaceAll("%DESC%", esc(g.description));
  writeFileSync(join(out, g.id, "index.html"), html);
}
console.log(`site/dist ready (${GENERATORS.length + 1} generator pages)`);
