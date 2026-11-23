// A small pool of module workers that render generator images. Jobs carry a tag so that stale work can
// be dropped from the queue before it starts. Falls back to the main thread when module workers are
// not available. © 2026 Peter Alexeev Dauwalter

let inline = null;
async function renderInline(job) {
  if (!inline) inline = await import("/swirlseed/index.js");
  const t = performance.now();
  const px = job.gen === "swirl" ? inline.generate(job.seed, job.w, job.h, job.params || {}) : inline.renderGenerator(job.gen, job.seed, job.w, job.h, job.params || {});
  return { ok: true, w: job.w, h: job.h, px, ms: performance.now() - t };
}

export class Pool {
  constructor(n) {
    this.q = [];
    this.slots = [];
    for (let i = 0; i < n; i++) {
      let w = null;
      try {
        w = new Worker(new URL("/app/worker.js", location.origin), { type: "module" });
      } catch {
        w = null;
      }
      const slot = { w, task: null };
      if (w) {
        w.onmessage = (e) => this.done(slot, e.data);
        w.onerror = () => {
          // module workers unsupported or the worker failed to load: render on the main thread
          slot.w = null;
          if (slot.task) {
            const t = slot.task;
            slot.task = null;
            this.q.unshift(t);
          }
          this.pump();
        };
      }
      this.slots.push(slot);
    }
  }
  /** Queue a job; resolves with {w, h, px, ms} or null when cancelled. */
  run(job, tag = "", front = false) {
    return new Promise((res) => {
      const t = { job, tag, res };
      if (front) this.q.unshift(t);
      else this.q.push(t);
      this.pump();
    });
  }
  /** Drop queued (not yet started) jobs with this tag. */
  cancel(tag) {
    this.q = this.q.filter((t) => {
      if (t.tag !== tag) return true;
      t.res(null);
      return false;
    });
  }
  done(slot, r) {
    const t = slot.task;
    slot.task = null;
    if (t) t.res(r.ok ? r : null);
    if (!r.ok) console.warn(r.error);
    this.pump();
  }
  pump() {
    for (const slot of this.slots) {
      if (slot.task || !this.q.length) continue;
      const t = this.q.shift();
      slot.task = t;
      if (slot.w) slot.w.postMessage(t.job);
      else
        setTimeout(() => {
          renderInline(t.job).then(
            (r) => this.done(slot, r),
            (e) => this.done(slot, { ok: false, error: String(e) }),
          );
        }, 0);
    }
  }
}

/** Put a rendered result into a canvas. */
export function paint(canvas, r) {
  if (!r) return;
  if (canvas.width !== r.w) canvas.width = r.w;
  if (canvas.height !== r.h) canvas.height = r.h;
  canvas.getContext("2d").putImageData(new ImageData(r.px, r.w, r.h), 0, 0);
}

/** Width and height for an aspect ratio (w / h) with the given long side. */
export function dims(aspect, long) {
  return aspect >= 1 ? [long, Math.max(16, Math.round(long / aspect))] : [Math.max(16, Math.round(long * aspect)), long];
}

const W1 = ["amber", "glass", "tidal", "velvet", "neon", "ember", "lunar", "coral", "moss", "indigo", "opal", "dusk"];
const W2 = ["river", "bloom", "drift", "harbor", "echo", "canyon", "aurora", "orbit", "silk", "storm", "garden", "signal"];
export function randomSeed() {
  return W1[(Math.random() * W1.length) | 0] + " " + W2[(Math.random() * W2.length) | 0] + " " + ((Math.random() * 90 + 10) | 0);
}

/** The seed shared between the catalogue and the generator pages. */
export function sharedSeed() {
  const q = new URLSearchParams(location.search).get("seed");
  if (q !== null) return q;
  try {
    return sessionStorage.getItem("swirlseed.seed") || "Hello, world!";
  } catch {
    return "Hello, world!";
  }
}
export function rememberSeed(s) {
  try {
    sessionStorage.setItem("swirlseed.seed", s);
  } catch {
    /* private mode */
  }
}
