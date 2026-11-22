# swirlseed

Deterministic fluid, glassy marbling patterns generated from a string seed. Same text → same RGBA pixels, byte for byte, and identical to the [`swirlseed` npm package](https://github.com/alexeev-dauwalter/swirlseed/tree/main/packages/swirlseed).

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

## Features

* `nfc` (default) — NFC-normalise the seed so that `é` typed as one or two code points gives the same picture. Pulls in `unicode-normalization`. Without it the library has no dependencies; ASCII and already-normalised text behave identically either way.

Full documentation, the algorithm and the golden-test setup are in the [repository README](https://github.com/alexeev-dauwalter/swirlseed#readme).
