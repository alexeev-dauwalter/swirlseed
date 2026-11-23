export { createPattern, paletteHex } from "./params.js";
export type { Mood, PatternParams, Stop, WarpLayer } from "./params.js";
export { generate, renderPattern } from "./render.js";
export type { DitherOptions, GrainOptions, HalftoneOptions, RenderOptions, RibsOptions } from "./options.js";

import { generate } from "./render.js";
import type { RenderOptions } from "./options.js";

/** Browser helper: draw a pattern into a canvas (size taken from `canvas.width/height`). */
export function drawToCanvas(canvas: HTMLCanvasElement, seed: string, options: RenderOptions = {}): void {
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("2D canvas context is not available");
  const px = generate(seed, canvas.width, canvas.height, options);
  ctx.putImageData(new ImageData(px as Uint8ClampedArray<ArrayBuffer>, canvas.width, canvas.height), 0, 0);
}

export { GENERATORS, getGenerator, renderGenerator, resolveParams } from "./gen/index.js";
export type { Generator, Param, NumberParam, ChoiceParam, ParamValue, ParamValues, Preview } from "./gen/index.js";

import { renderGenerator } from "./gen/index.js";
import type { ParamValues } from "./gen/index.js";

/** Browser helper: draw any generator into a canvas (size taken from `canvas.width/height`). */
export function drawGenerator(canvas: HTMLCanvasElement, id: string, seed: string, params: ParamValues = {}): void {
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("2D canvas context is not available");
  const px = renderGenerator(id, seed, canvas.width, canvas.height, params);
  ctx.putImageData(new ImageData(px as Uint8ClampedArray<ArrayBuffer>, canvas.width, canvas.height), 0, 0);
}
