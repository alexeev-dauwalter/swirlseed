//! # swirlseed
//!
//! Deterministic fluid, glassy marbling patterns generated from a string seed.
//! The same seed gives byte-identical pixels here and in the `swirlseed` npm package.
//!
//! ```
//! use swirlseed::{generate, RenderOptions};
//!
//! let rgba = generate("Hello, world!", 256, 256, &RenderOptions::default());
//! assert_eq!(rgba.len(), 256 * 256 * 4);
//! ```
//!
//! Pass the pieces separately when you want to inspect or reuse the parameters:
//!
//! ```
//! use swirlseed::{create_pattern, palette_hex, render_pattern, RenderOptions};
//!
//! let params = create_pattern("Hello, world!");
//! println!("{} {:?}", params.mood.as_str(), palette_hex(&params));
//! let rgba = render_pattern(&params, 128, 128, &RenderOptions::default());
//! # assert_eq!(rgba.len(), 128 * 128 * 4);
//! ```

// The code mirrors the TypeScript implementation expression by expression (that is what keeps the output
// byte-identical), so a few style lints that would restructure it are off.
#![allow(clippy::manual_clamp, clippy::needless_range_loop)]

pub mod color;
pub mod dmath;
pub mod gen;
mod halftone;
mod noise;
pub mod options;
pub mod params;
pub mod render;
pub mod rng;

pub use gen::{get_generator, render_generator, Generator, Params as GenParams, Value as GenValue, GENERATORS};
pub use options::{
    DitherOptions, GrainOptions, HalftoneMode, HalftoneOptions, HalftoneShape, RenderOptions, RibsOptions,
};
pub use params::{create_pattern, palette_hex, Mode, Mood, Params, Stop, Swirl, WarpLayer};
pub use render::{generate, render_pattern};
