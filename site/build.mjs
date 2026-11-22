// Assembles the static site into site/dist: the page from site/src plus the compiled
// browser build of the library (packages/swirlseed/dist, plain ES modules).
import { cpSync, existsSync, mkdirSync, readdirSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const lib = join(here, "..", "packages", "swirlseed", "dist");
const out = join(here, "dist");

if (!existsSync(join(lib, "index.js"))) {
  console.error("packages/swirlseed/dist is missing. Run `npm install && npm run build` in packages/swirlseed first.");
  process.exit(1);
}
rmSync(out, { recursive: true, force: true });
cpSync(join(here, "src"), out, { recursive: true });
mkdirSync(join(out, "swirlseed"), { recursive: true });
for (const f of readdirSync(lib)) {
  if (f.endsWith(".js")) cpSync(join(lib, f), join(out, "swirlseed", f));
}
console.log("site/dist ready");
