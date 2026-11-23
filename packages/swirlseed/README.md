# swirlseed

Deterministic generative images from a string seed: fluid, glassy marbling plus a physically based sky, ebru, suminagashi, mesh gradient, flow field, watercolour, wet-on-wet, aurora, planets and nebulae, dendritic painting, paste paper and metaballs. Same text → same RGBA pixels, byte for byte, in every JS runtime and identical to the [`swirlseed` Rust crate](https://github.com/alexeev-dauwalter/swirlseed/tree/main/crates/swirlseed). Zero dependencies. Demo: [swirlseed.41t.dev](https://swirlseed.41t.dev).

```ts
import { generate, drawToCanvas, createPattern, paletteHex } from "swirlseed";

// RGBA bytes, Uint8ClampedArray(width * height * 4)
const rgba = generate("Hello, world!", 1024, 1024, { halftone: { shape: "dot" }, grain: {} });

// browser: straight into a canvas (size = canvas.width × canvas.height)
drawToCanvas(document.querySelector("canvas")!, "Hello, world!", { ribs: { count: 24 } });

// what the seed decided
const params = createPattern("Hello, world!");
console.log(params.mood, paletteHex(params));
```

Effects (`ribs`, `halftone`, `dither`, `grain`) and `shine` are independent of the seed and off by default; pass `{}` to enable an effect with its defaults. Angles are in radians. See the `RenderOptions` type for every field.

## Generators

```ts
import { renderGenerator, drawGenerator, getGenerator, GENERATORS } from "swirlseed";

const rgba = renderGenerator("sky", "evening", 1600, 1000, { time: 18.4, type: "cirrus" });
drawGenerator(canvas, "metaballs", "Hello, world!", { look: "glass" });

GENERATORS.map((g) => g.id); // sky, ebru, suminagashi, mesh-gradient, flow-field, watercolor, …
getGenerator("planets").params; // ranges, defaults and choices: enough to build a UI
```

Missing parameters take their defaults; unknown keys and choices throw. Rendering is synchronous and CPU-bound (tens of milliseconds to a second or two depending on the generator and size), so run it in a Web Worker for interactive pages.

Full documentation, the algorithm and the golden-test setup are in the [repository README](https://github.com/alexeev-dauwalter/swirlseed#readme).
