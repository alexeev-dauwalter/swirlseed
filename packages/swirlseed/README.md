# swirlseed

Deterministic fluid, glassy marbling patterns generated from a string seed. Same text → same RGBA pixels, byte for byte, in every JS runtime and identical to the [`swirlseed` Rust crate](https://github.com/alexeev-dauwalter/swirlseed/tree/main/crates/swirlseed). Zero dependencies.

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

Full documentation, the algorithm and the golden-test setup are in the [repository README](https://github.com/alexeev-dauwalter/swirlseed#readme).
