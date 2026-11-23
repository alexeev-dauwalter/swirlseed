// A generator page built from the generator's parameter spec: live view, controls, variation strips
// and code snippets. © 2026 Peter Alexeev Dauwalter
import { getGenerator } from "/swirlseed/index.js";
import { Pool, dims, paint, randomSeed, rememberSeed, sharedSeed } from "/app/pool.js";
import { cliWithSeed, hl } from "/app/code.js";

const g = getGenerator(document.body.dataset.gen);
const $ = (s) => document.querySelector(s);
const canvas = $("#gcanvas");
const cores = navigator.hardwareConcurrency || 4;
const view = new Pool(1);
const thumbs = new Pool(Math.max(1, Math.min(3, cores - 2)));

const LOW = 480; // long side while you drag
const HIGH = 1200; // long side once you stop
const DOWNLOAD = 2400; // long side of the PNG
const THUMB = 300;
const FRAMES = [
  ["Native", g.aspect],
  ["1:1", 1],
  ["16:9", 16 / 9],
  ["4:5", 4 / 5],
];

// ---------- state (seed shared with the catalogue, params in the URL) ----------
const defaults = {};
for (const p of g.params) defaults[p.key] = p.default;
const state = { seed: sharedSeed(), frame: 0, p: { ...defaults } };
{
  const q = new URLSearchParams(location.search);
  for (const p of g.params) {
    const v = q.get(p.key);
    if (v === null) continue;
    if (p.kind === "number") {
      const n = Number(v);
      if (Number.isFinite(n)) state.p[p.key] = Math.min(p.max, Math.max(p.min, n));
    } else {
      const o = p.options.find((o) => o.value === v.toLowerCase());
      if (o) state.p[p.key] = o.value;
    }
  }
  const fi = FRAMES.findIndex((f) => f[0] === q.get("frame"));
  if (fi > 0) state.frame = fi;
}
const changed = () => g.params.filter((p) => state.p[p.key] !== p.default);

function show(p, v) {
  switch (p.unit) {
    case "%":
      return Math.round(v * 100) + "%";
    case "x":
      return +(+v).toFixed(2) + "×";
    case "°":
      return Math.round(v) + "°";
    case "h": {
      const m = Math.round(v * 60) % 1440;
      return String(Math.floor(m / 60)).padStart(2, "0") + ":" + String(m % 60).padStart(2, "0");
    }
    default:
      return String(+(+v).toFixed(4));
  }
}
const num = (v) => String(+(+v).toFixed(6));

// ---------- controls ----------
const host = $("#gparams");
for (const p of g.params) {
  const el = document.createElement("div");
  el.className = "field";
  el.dataset.k = p.key;
  if (p.kind === "number") {
    el.innerHTML = '<div class="prow"><label></label><output></output></div><input type="range">';
    el.querySelector("label").textContent = p.label;
    const r = el.querySelector("input");
    r.min = p.min;
    r.max = p.max;
    r.step = p.step;
    r.setAttribute("aria-label", p.label);
    r.addEventListener("input", () => {
      state.p[p.key] = +r.value;
      el.querySelector("output").textContent = show(p, +r.value);
      update();
    });
  } else {
    el.innerHTML = "<label></label>";
    el.querySelector("label").textContent = p.label;
    const groups = [];
    for (const o of p.options) if (o.group && !groups.includes(o.group)) groups.push(o.group);
    const sets = groups.length ? groups.map((gr) => [gr, p.options.filter((o) => o.group === gr)]) : [["", p.options]];
    for (const [gr, opts] of sets) {
      if (gr) el.insertAdjacentHTML("beforeend", '<div class="sublabel">' + gr + "</div>");
      const seg = document.createElement("div");
      seg.className = opts.length <= 4 && !gr ? "seg" : "seg wrap";
      for (const o of opts) {
        const b = document.createElement("button");
        b.type = "button";
        b.textContent = o.label;
        b.dataset.v = o.value;
        b.onclick = () => {
          state.p[p.key] = o.value;
          sync();
          update();
        };
        seg.appendChild(b);
      }
      el.appendChild(seg);
    }
  }
  host.appendChild(el);
}
const sizeHost = $("#gsize");
FRAMES.forEach((f, i) => {
  const b = document.createElement("button");
  b.type = "button";
  b.textContent = f[0];
  b.onclick = () => {
    state.frame = i;
    sync();
    update(true);
  };
  sizeHost.appendChild(b);
});

function sync() {
  $("#gseed").value = state.seed;
  [...sizeHost.children].forEach((b, i) => b.classList.toggle("on", i === state.frame));
  for (const p of g.params) {
    const el = host.querySelector('[data-k="' + p.key + '"]');
    const v = state.p[p.key];
    if (p.kind === "number") {
      el.querySelector("input").value = v;
      el.querySelector("output").textContent = show(p, v);
    } else el.querySelectorAll("button").forEach((b) => b.classList.toggle("on", b.dataset.v === v));
  }
  stripMarks();
}

// ---------- view ----------
let ver = 0;
let playing = false;
const playBtn = $("#gplay");
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const frameDims = (long) => dims(FRAMES[state.frame][1], long);
const job = (w, h, params) => ({ gen: g.id, seed: state.seed, w, h, params: params || { ...state.p } });
function info(r) {
  $("#ginfo").textContent = g.title + " · " + r.w + "×" + r.h + " · " + Math.round(r.ms) + " ms";
}
async function renderView() {
  const my = ++ver;
  view.cancel("view");
  const lo = await view.run(job(...frameDims(LOW)), "view");
  if (!lo || my !== ver) return;
  paint(canvas, lo);
  info(lo);
  await sleep(320);
  if (my !== ver) return;
  const hi = await view.run(job(...frameDims(HIGH)), "view");
  if (!hi || my !== ver) return;
  paint(canvas, hi);
  info(hi);
}

// ---------- variation strips ----------
const stripHost = $("#strips");
const stripItems = [];
for (const pv of g.previews || []) {
  const p = g.params.find((q) => q.key === pv.key);
  const s = document.createElement("div");
  s.className = "strip";
  s.innerHTML = '<div class="striphead"><h3></h3><p></p></div><div class="strip-row"></div>';
  s.querySelector("h3").textContent = pv.title;
  s.querySelector("p").textContent = pv.sub;
  const row = s.querySelector(".strip-row");
  row.style.setProperty("--n", pv.values.length);
  pv.values.forEach((v, i) => {
    const b = document.createElement("button");
    b.type = "button";
    b.className = "thumb";
    b.innerHTML = "<canvas></canvas><span></span>";
    b.querySelector("span").textContent = pv.labels[i];
    b.onclick = () => {
      state.p[pv.key] = v;
      sync();
      update();
      window.scrollTo({ top: 0, behavior: "smooth" });
    };
    row.appendChild(b);
    stripItems.push({ key: pv.key, v, b, c: b.querySelector("canvas"), p });
  });
  stripHost.appendChild(s);
}
function stripMarks() {
  for (const it of stripItems) it.b.classList.toggle("on", state.p[it.key] === it.v);
}
let sver = 0;
let stm = 0;
function renderStrips() {
  const my = ++sver;
  thumbs.cancel("strip");
  const [w, h] = frameDims(THUMB);
  for (const it of stripItems) {
    it.b.classList.add("busy");
    thumbs.run(job(w, h, { ...state.p, [it.key]: it.v }), "strip").then((r) => {
      if (!r || my !== sver) return;
      paint(it.c, r);
      it.b.classList.remove("busy");
    });
  }
}

// ---------- code ----------
const f1 = (v) => {
  const t = num(v);
  return t.includes(".") || t.includes("e") ? t : t + ".0";
};
function cli() {
  const [w, h] = frameDims(DOWNLOAD);
  let c = "swirlseed -g " + g.id + " -s " + w + "x" + h;
  const ch = changed();
  if (ch.length) c += " \\\n  " + ch.map((p) => "-p " + p.key + "=" + (p.kind === "number" ? num(state.p[p.key]) : state.p[p.key])).join(" ");
  return cliWithSeed(c + " -o " + g.id + ".png", state.seed);
}
function ts() {
  const [w, h] = frameDims(DOWNLOAD);
  const ch = changed();
  const obj = ch.length ? ", { " + ch.map((p) => p.key + ": " + (p.kind === "number" ? num(state.p[p.key]) : JSON.stringify(state.p[p.key]))).join(", ") + " }" : "";
  return 'import { drawGenerator } from "swirlseed";\n\ncanvas.width = ' + w + ";\ncanvas.height = " + h + ";\ndrawGenerator(canvas, " + JSON.stringify(g.id) + ", " + JSON.stringify(state.seed) + obj + ");";
}
function rs() {
  const [w, h] = frameDims(DOWNLOAD);
  const ch = changed();
  let pre = "";
  let arg = "&[]";
  if (ch.length) {
    pre =
      "let params = [\n" +
      ch.map((p) => "    (" + JSON.stringify(p.key) + ".to_string(), " + (p.kind === "number" ? "Value::Num(" + f1(state.p[p.key]) + ")" : "Value::Str(" + JSON.stringify(state.p[p.key]) + ".into())") + "),").join("\n") +
      "\n];\n";
    arg = "&params";
  }
  return (
    "use swirlseed::gen::{render_generator" + (ch.length ? ", Value" : "") + "};\n\n" + pre +
    "let rgba: Vec<u8> = render_generator(" + JSON.stringify(g.id) + ", " + JSON.stringify(state.seed) + ", " + w + ", " + h + ", " + arg + ")?;\nassert_eq!(rgba.len(), " + w + " * " + h + " * 4);"
  );
}
let lang = "cli";
let codeText = "";
function renderCode() {
  codeText = lang === "cli" ? cli() : lang === "ts" ? ts() : rs();
  $("#gcode").innerHTML = hl(codeText, lang);
}
document.querySelectorAll("#gtabs button").forEach((b) => {
  b.onclick = () => {
    lang = b.dataset.lang;
    document.querySelectorAll("#gtabs button").forEach((x) => x.classList.toggle("on", x === b));
    renderCode();
  };
});
$("#gcopy").onclick = function () {
  try {
    navigator.clipboard.writeText(codeText);
  } catch {
    /* no clipboard */
  }
  this.textContent = "Copied";
  setTimeout(() => (this.textContent = "Copy"), 1200);
};

// ---------- wiring ----------
function writeUrl() {
  const u = new URL(location.href);
  u.search = "";
  u.searchParams.set("seed", state.seed);
  if (state.frame) u.searchParams.set("frame", FRAMES[state.frame][0]);
  for (const p of changed()) u.searchParams.set(p.key, p.kind === "number" ? num(state.p[p.key]) : state.p[p.key]);
  history.replaceState(null, "", u);
}
function update(now) {
  if (playing) {
    playing = false;
    playBtn.textContent = "Play";
  }
  writeUrl();
  renderCode();
  stripMarks();
  renderView();
  clearTimeout(stm);
  stm = setTimeout(renderStrips, now ? 0 : 450);
}
$("#gseed").addEventListener("input", (e) => {
  state.seed = e.target.value;
  rememberSeed(state.seed);
  document.dispatchEvent(new Event("seedchange"));
  update();
});
$("#gdice").onclick = () => {
  state.seed = randomSeed();
  rememberSeed(state.seed);
  sync();
  document.dispatchEvent(new Event("seedchange"));
  update(true);
};
$("#greset").onclick = () => {
  state.p = { ...defaults };
  sync();
  update(true);
};
$("#gdl").onclick = async function () {
  const b = this;
  const [w, h] = frameDims(DOWNLOAD);
  b.disabled = true;
  b.textContent = "Rendering " + w + "×" + h + "…";
  const r = await view.run(job(w, h), "download", true);
  b.disabled = false;
  b.textContent = "Download PNG";
  if (!r) return;
  const c = document.createElement("canvas");
  paint(c, r);
  c.toBlob((blob) => {
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    const slug = state.seed.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "seed";
    a.download = g.id + "-" + slug + ".png";
    a.click();
    setTimeout(() => URL.revokeObjectURL(a.href), 1000);
  });
};

// sky: let the day run
const timeParam = g.params.find((p) => p.key === "time" && p.unit === "h");
if (timeParam) {
  playBtn.hidden = false;
  playBtn.onclick = async () => {
    if (playing) return update(true);
    playing = true;
    playBtn.textContent = "Pause";
    const playback = ++ver;
    view.cancel("view");
    const tEl = host.querySelector('[data-k="time"]');
    while (playback === ver) {
      state.p.time = +((state.p.time + 0.1) % 24).toFixed(2);
      tEl.querySelector("input").value = state.p.time;
      tEl.querySelector("output").textContent = show(timeParam, state.p.time);
      const r = await view.run(job(...frameDims(LOW)), "view");
      if (playback !== ver) return;
      if (!r) return update(true);
      paint(canvas, r);
      info(r);
      await new Promise((res) => requestAnimationFrame(res));
    }
  };
  host.querySelector('[data-k="time"] input').addEventListener("pointerdown", () => {
    if (playing) update(true);
  });
}

sync();
update(true);
