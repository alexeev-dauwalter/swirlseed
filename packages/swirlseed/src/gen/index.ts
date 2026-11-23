import { type Generator, type ParamValues, resolveParams } from "./spec.js";
import { sky } from "./sky.js";
import { ebru, suminagashi } from "./marble.js";
import { mesh } from "./mesh.js";
import { flow } from "./flow.js";
import { aurora } from "./aurora.js";
import { planets } from "./planets.js";
import { metaballs } from "./metaballs.js";
import { watercolor, wetOnWet, pastePaper, dendritic } from "./paint.js";

export type { Generator, Param, NumberParam, ChoiceParam, ParamValue, ParamValues, Preview } from "./spec.js";
export { resolveParams } from "./spec.js";

/** Every generator in the package, in catalogue order. */
export const GENERATORS: Generator[] = [sky, ebru, suminagashi, mesh, flow, watercolor, wetOnWet, aurora, planets, dendritic, pastePaper, metaballs];

export function getGenerator(id: string): Generator {
  const g = GENERATORS.find((q) => q.id === id.toLowerCase());
  if (!g) throw new Error(`unknown generator "${id}"; available: ${GENERATORS.map((q) => q.id).join(", ")}`);
  return g;
}

/** Render a generator to RGBA bytes (`width * height * 4`). Missing parameters take their defaults. */
export function renderGenerator(id: string, seed: string, width: number, height: number, params: ParamValues = {}): Uint8ClampedArray {
  const g = getGenerator(id);
  return g.render(seed, width, height, resolveParams(g, params));
}
