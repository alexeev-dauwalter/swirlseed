/** Reeded ("fluted") glass. */
export interface RibsOptions {
  /** Number of flutes across the frame height (default 24). */
  count?: number;
  /** Lens refraction strength, 0…1.5 (default 0.6). */
  strength?: number;
  /** Direction in radians; 0 = vertical flutes (default 0). */
  angle?: number;
  /** Shading of the flute faces, 0…1 (default 0.5). */
  shade?: number;
}

/** Halftone. Sizes are in px for a 512 px frame and scale with the actual frame size. */
export interface HalftoneOptions {
  /** Cell pitch (default 10). */
  size?: number;
  /** Grid angle in radians (default 0.5236, i.e. 30°). */
  angle?: number;
  shape?: "dot" | "square" | "line";
  /** `color`: dots take the pattern colour on "paper"; `mono`: a single ink on paper. */
  mode?: "color" | "mono";
  /** false: dark ink on light paper; true: glowing dots on a dark background. */
  invert?: boolean;
  /** Blend with the untouched pattern, 0…1 (default 1). */
  mix?: number;
}

/** Ordered Bayer 8×8 dithering. */
export interface DitherOptions {
  /** Levels per channel, 2…32 (default 4). */
  levels?: number;
  /** Size of one matrix cell in px for a 512 px frame (default 1.5). */
  scale?: number;
  /** Strength of the matrix pattern, 0…1 (default 1). */
  amount?: number;
}

/** Film grain. */
export interface GrainOptions {
  /** 0…1 (default 0.35). */
  amount?: number;
  /** Grain size in px for a 512 px frame (default 1). */
  size?: number;
}

/**
 * Texture and lighting options. They are independent of the seed and off by default.
 * Pass `{}` for an effect to enable it with its defaults.
 */
export interface RenderOptions {
  /** 0…1.5. When omitted, `params.shine` (derived from the seed) is used. 0 disables lighting. */
  shine?: number;
  ribs?: RibsOptions | null;
  halftone?: HalftoneOptions | null;
  dither?: DitherOptions | null;
  grain?: GrainOptions | null;
}
