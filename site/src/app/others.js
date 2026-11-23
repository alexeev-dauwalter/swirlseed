// "More generators" links at the bottom of every generator page. © 2026 Peter Alexeev Dauwalter
import { GENERATORS } from "/swirlseed/index.js";
import { rememberSeed, sharedSeed } from "/app/pool.js";

const current = document.body.dataset.gen;
const host = document.getElementById("others");
const all = [{ id: "swirl", title: "Swirl" }, ...GENERATORS];
function seedNow() {
  const el = document.getElementById("gseed");
  return el ? el.value : sharedSeed();
}
function draw() {
  const s = seedNow();
  host.innerHTML = '<span class="olabel">More generators</span>';
  for (const g of all) {
    if (g.id === current) continue;
    const a = document.createElement("a");
    a.href = "/" + g.id + "/" + (s ? "?seed=" + encodeURIComponent(s) : "");
    a.textContent = g.title;
    host.appendChild(a);
  }
}
draw();
document.addEventListener("seedchange", draw);
const seedEl = document.getElementById("gseed");
if (seedEl && current === "swirl") {
  // the swirl page keeps its own script; share its seed with the other pages
  const sync = () => {
    rememberSeed(seedEl.value);
    const u = new URL(location.href);
    u.searchParams.set("seed", seedEl.value);
    history.replaceState(null, "", u);
    draw();
  };
  seedEl.addEventListener("input", sync);
  document.getElementById("gdice")?.addEventListener("click", () => setTimeout(sync, 0));
}
