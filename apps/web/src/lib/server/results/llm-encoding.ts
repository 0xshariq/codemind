import { flatten, unflatten } from "flat";
import { encode, decode } from "@toon-format/toon";
import { isDeepStrictEqual } from "node:util";
import { projectForLlm } from "./llm-projection";

export function encodeLlmResult(value: Record<string, unknown>, maxBytes = 256 * 1024): string {
  assertFlattenable(value);
  let projected = projectForLlm(value);
  let encoded = encodeFlattened(projected);
  while (Buffer.byteLength(encoded, "utf8") > maxBytes) {
    const next = projectForLlm(projected, Math.max(1, Math.floor(
      Math.max(1, countProjectedItems(projected)) / 2,
    )));
    if (JSON.stringify(next) === JSON.stringify(projected)) {
      throw new Error("LLM result exceeds the configured encoding budget");
    }
    projected = { ...next, truncated: true, completeness: "partial" };
    encoded = encodeFlattened(projected);
  }
  return encoded;
}

function encodeFlattened(projected: Record<string, unknown>): string {
  const flattened = flatten(projected, { safe: true });
  // `object: true` forces numeric array indices to become object keys in flat v6,
  // which loses canonical array semantics across the LLM boundary.
  const restored = unflatten(flattened);
  if (!isDeepStrictEqual(restored, projected)) {
    throw new Error("LLM result flattening was not reversible");
  }
  return encode(flattened);
}

function assertFlattenable(value: unknown, path = "result"): void {
  if (Array.isArray(value)) {
    value.forEach((item, index) => assertFlattenable(item, `${path}[${index}]`));
    return;
  }
  if (!value || typeof value !== "object") return;
  for (const [key, item] of Object.entries(value)) {
    if (key.includes(".") || key.includes("[") || key.includes("]") || ["__proto__", "prototype", "constructor"].includes(key)
      || /[\u0000-\u001f\u007f]/.test(key)) {
      throw new Error(`LLM result contains an ambiguous key at ${path}`);
    }
    assertFlattenable(item, `${path}.${key}`);
  }
}

function countProjectedItems(value: Record<string, unknown>): number {
  return Object.values(value).reduce(
    (count: number, item) => count + (Array.isArray(item) ? item.length : 0),
    0,
  );
}

export function decodeLlmResult(value: string): Record<string, unknown> {
  return unflatten(decode(value) as Record<string, unknown>) as Record<string, unknown>;
}
