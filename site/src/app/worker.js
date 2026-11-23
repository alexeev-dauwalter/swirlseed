// Renders one image per message off the main thread. © 2026 Peter Alexeev Dauwalter
import { generate, renderGenerator } from "/swirlseed/index.js";

self.onmessage = (e) => {
  const { gen, seed, w, h, params } = e.data;
  try {
    const t = performance.now();
    const px = gen === "swirl" ? generate(seed, w, h, params || {}) : renderGenerator(gen, seed, w, h, params || {});
    self.postMessage({ ok: true, w, h, px, ms: performance.now() - t }, [px.buffer]);
  } catch (err) {
    self.postMessage({ ok: false, error: String(err && err.message ? err.message : err) });
  }
};
