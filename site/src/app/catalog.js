// Home page: one shared seed, a live preview of every generator. © 2026 Peter Alexeev Dauwalter
import { GENERATORS } from "/swirlseed/index.js";
import { Pool, paint, randomSeed, rememberSeed, sharedSeed } from "/app/pool.js";

const SWIRL = {
  id: "swirl",
  title: "Swirl",
  description: "The original: fluid, glassy marbling with reeded glass, halftone, dither and grain.",
  params: [{}, {}, {}, {}, {}],
};
const ALL = [SWIRL, ...GENERATORS];
const TAGS = {
  swirl: "5 effects",
  sky: "physically based",
  metaballs: "3D · 2D",
  planets: "5 types",
  ebru: "5 styles",
  suminagashi: "floating ink",
  "mesh-gradient": "OKLab",
  "flow-field": "streamlines",
  watercolor: "pigment & paper",
  "wet-on-wet": "pigment & paper",
  aurora: "emission",
  "dendritic-painting": "DLA growth",
  "paste-paper": "relief",
};

const grid = document.getElementById("catalog");
const input = document.getElementById("cseed");
const pool = new Pool(Math.max(2, Math.min(4, (navigator.hardwareConcurrency || 4) - 1)));
let seed = sharedSeed();
input.value = seed;

const cards = ALL.map((g) => {
  const a = document.createElement("a");
  a.className = g.id === "swirl" ? "gcard hero" : "gcard";
  a.dataset.gen = g.id;
  a.innerHTML =
    '<div class="gthumb"><canvas width="480" height="320"></canvas></div>' +
    '<div class="gtext"><div class="gtitle"><h3></h3><span class="gtag"></span></div><p></p></div>';
  a.querySelector("h3").textContent = g.title;
  a.querySelector("p").textContent = g.description;
  a.querySelector(".gtag").textContent = TAGS[g.id] || g.params.length + " params";
  grid.appendChild(a);
  return { g, a, c: a.querySelector("canvas") };
});

function links() {
  for (const k of cards) k.a.href = "/" + k.g.id + "/" + (seed ? "?seed=" + encodeURIComponent(seed) : "");
}

let gen = 0;
function renderAll() {
  const my = ++gen;
  pool.cancel("cards");
  links();
  // visible cards first
  const vh = innerHeight;
  const order = cards.slice().sort((x, y) => {
    const a = x.a.getBoundingClientRect().top;
    const b = y.a.getBoundingClientRect().top;
    return (a > vh) - (b > vh) || a - b;
  });
  for (const k of order) {
    k.a.classList.add("busy");
    let w = 480;
    let h = 320;
    if (k.a.classList.contains("hero")) {
      // the featured card is as large as four: render it at its own size
      const b = k.c.parentElement.getBoundingClientRect();
      const s = Math.min(2, devicePixelRatio || 1);
      w = Math.max(320, Math.min(1400, Math.round(b.width * s)));
      h = Math.max(200, Math.min(1400, Math.round(b.height * s)));
    }
    pool.run({ gen: k.g.id, seed, w, h, params: k.g.id === "swirl" ? { shine: 0.75 } : {} }, "cards").then((r) => {
      if (!r || my !== gen) return;
      paint(k.c, r);
      k.a.classList.remove("busy");
    });
  }
}

let tm = 0;
function setSeed(s, now) {
  seed = s;
  rememberSeed(s);
  const u = new URL(location.href);
  u.searchParams.set("seed", s);
  history.replaceState(null, "", u);
  clearTimeout(tm);
  if (now) renderAll();
  else tm = setTimeout(renderAll, 260);
}
input.addEventListener("input", () => setSeed(input.value, false));
document.getElementById("cform").addEventListener("submit", (e) => {
  e.preventDefault();
  setSeed(input.value, true);
});
document.getElementById("cdice").addEventListener("click", () => {
  input.value = randomSeed();
  setSeed(input.value, true);
});
renderAll();
