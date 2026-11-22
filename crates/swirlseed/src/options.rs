//! Texture and lighting options. Mirrors `options.ts`.
//!
//! They are independent of the seed and off by default. `Some(XOptions::default())` enables an
//! effect with its defaults (every field is optional, like in the TypeScript package).

/// Reeded ("fluted") glass.
#[derive(Clone, Debug, Default)]
pub struct RibsOptions {
    /// Number of flutes across the frame height (default 24).
    pub count: Option<f64>,
    /// Lens refraction strength, 0..1.5 (default 0.6).
    pub strength: Option<f64>,
    /// Direction in radians; 0 = vertical flutes (default 0).
    pub angle: Option<f64>,
    /// Shading of the flute faces, 0..1 (default 0.5).
    pub shade: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HalftoneShape {
    #[default]
    Dot,
    Square,
    Line,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HalftoneMode {
    /// Dots take the pattern colour on "paper".
    #[default]
    Color,
    /// A single ink on paper.
    Mono,
}

/// Halftone. Sizes are in px for a 512 px frame and scale with the actual frame size.
#[derive(Clone, Debug, Default)]
pub struct HalftoneOptions {
    /// Cell pitch (default 10).
    pub size: Option<f64>,
    /// Grid angle in radians (default 0.5236, i.e. 30 degrees).
    pub angle: Option<f64>,
    pub shape: Option<HalftoneShape>,
    pub mode: Option<HalftoneMode>,
    /// false: dark ink on light paper; true: glowing dots on a dark background.
    pub invert: Option<bool>,
    /// Blend with the untouched pattern, 0..1 (default 1).
    pub mix: Option<f64>,
}

/// Ordered Bayer 8x8 dithering.
#[derive(Clone, Debug, Default)]
pub struct DitherOptions {
    /// Levels per channel, 2..32 (default 4).
    pub levels: Option<f64>,
    /// Size of one matrix cell in px for a 512 px frame (default 1.5).
    pub scale: Option<f64>,
    /// Strength of the matrix pattern, 0..1 (default 1).
    pub amount: Option<f64>,
}

/// Film grain.
#[derive(Clone, Debug, Default)]
pub struct GrainOptions {
    /// 0..1 (default 0.35).
    pub amount: Option<f64>,
    /// Grain size in px for a 512 px frame (default 1).
    pub size: Option<f64>,
}

#[derive(Clone, Debug, Default)]
pub struct RenderOptions {
    /// 0..1.5. When `None`, `Params::shine` (derived from the seed) is used. 0 disables lighting.
    pub shine: Option<f64>,
    pub ribs: Option<RibsOptions>,
    pub halftone: Option<HalftoneOptions>,
    pub dither: Option<DitherOptions>,
    pub grain: Option<GrainOptions>,
}
