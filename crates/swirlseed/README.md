# swirlseed

Deterministic generative images from a string seed: fluid, glassy marbling plus a physically based sky, ebru, suminagashi, mesh gradient, flow field, watercolour, wet-on-wet, aurora, planets and nebulae, dendritic painting, paste paper and metaballs. Same text → same RGBA pixels, byte for byte, and identical to the [`swirlseed` npm package](https://github.com/alexeev-dauwalter/swirlseed/tree/main/packages/swirlseed). Demo: [swirlseed.41t.dev](https://swirlseed.41t.dev).

```rust
use swirlseed::{generate, HalftoneOptions, RenderOptions};

// plain pattern
let rgba = generate("Hello, world!", 1024, 1024, &RenderOptions::default());

// with an optional halftone texture
let opts = RenderOptions { halftone: Some(HalftoneOptions::default()), ..Default::default() };
let rgba = generate("Hello, world!", 1024, 1024, &opts);
assert_eq!(rgba.len(), 1024 * 1024 * 4);
```

Inspect what the seed decided:

```rust
let params = swirlseed::create_pattern("Hello, world!");
println!("{} {:?}", params.mood.as_str(), swirlseed::palette_hex(&params));
```

Effects (`ribs`, `halftone`, `dither`, `grain`) and `shine` are independent of the seed and off by default; see `RenderOptions`. To save a PNG from the command line use the `swirlseed-cli` crate.

## Generators

```rust
use swirlseed::gen::{get_generator, render_generator, Value, GENERATORS};

let params = [("time".to_string(), Value::Num(18.4)), ("type".to_string(), Value::Str("cirrus".into()))];
let rgba = render_generator("sky", "evening", 1600, 1000, &params)?; // Result<Vec<u8>, String>

for g in GENERATORS {
    println!("{} — {} ({} parameters)", g.id, g.title, g.params.len());
}
let spec = get_generator("metaballs")?; // ranges, defaults and choices of every parameter
```

Ids: `sky`, `ebru`, `suminagashi`, `mesh-gradient`, `flow-field`, `watercolor`, `wet-on-wet`, `aurora`, `planets`, `dendritic-painting`, `paste-paper`, `metaballs`. Missing parameters take their defaults; unknown keys and choices are errors.

## Features

* `nfc` (default) — NFC-normalise the seed so that `é` typed as one or two code points gives the same picture. Pulls in `unicode-normalization`. Without it the library has no dependencies; ASCII and already-normalised text behave identically either way.

Full documentation, the algorithm and the golden-test setup are in the [repository README](https://github.com/alexeev-dauwalter/swirlseed#readme).
