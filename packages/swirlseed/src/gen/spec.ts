/** Parameter descriptions shared by the API, the CLI and the demo site. */

export type ParamValue = number | string;
export type ParamValues = Record<string, ParamValue>;

export interface NumberParam {
  kind: "number";
  key: string;
  label: string;
  min: number;
  max: number;
  step: number;
  default: number;
  /** Display unit hint for UIs: "%", "°", "x", "h" or "". */
  unit?: string;
}
export interface ChoiceParam {
  kind: "choice";
  key: string;
  label: string;
  default: string;
  /** `group` lets a UI split long lists (e.g. metaball looks into "3D" and "2D"). */
  options: { value: string; label: string; group?: string }[];
}
export type Param = NumberParam | ChoiceParam;

export interface Preview {
  title: string;
  sub: string;
  key: string;
  values: ParamValue[];
  labels: string[];
}

export interface Generator {
  id: string;
  title: string;
  description: string;
  /** Default width / height for UIs. */
  aspect: number;
  params: Param[];
  previews: Preview[];
  render(seed: string, width: number, height: number, p: ParamValues): Uint8ClampedArray;
}

export const opt = (value: string, label?: string, group?: string): { value: string; label: string; group?: string } =>
  group ? { value, label: label ?? value, group } : { value, label: label ?? value };

/** Fill in defaults and check values. Choice values match case-insensitively. */
export function resolveParams(g: Generator, given: ParamValues = {}): ParamValues {
  const out: ParamValues = {};
  for (const p of g.params) {
    const v = given[p.key];
    if (p.kind === "number") {
      if (v === undefined) out[p.key] = p.default;
      else {
        const n = typeof v === "number" ? v : Number(v);
        if (!Number.isFinite(n)) throw new Error(`${g.id}: "${p.key}" must be a number`);
        out[p.key] = n;
      }
    } else {
      if (v === undefined) out[p.key] = p.default;
      else {
        const s = String(v).toLowerCase();
        const o = p.options.find((q) => q.value.toLowerCase() === s);
        if (!o) throw new Error(`${g.id}: "${p.key}" must be one of ${p.options.map((q) => q.value).join(", ")}`);
        out[p.key] = o.value;
      }
    }
  }
  for (const k of Object.keys(given)) if (!g.params.some((p) => p.key === k)) throw new Error(`${g.id}: unknown parameter "${k}"`);
  return out;
}
export const num = (p: ParamValues, k: string): number => p[k] as number;
export const str = (p: ParamValues, k: string): string => p[k] as string;
