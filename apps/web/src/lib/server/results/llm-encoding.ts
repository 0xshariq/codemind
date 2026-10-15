import { flatten, unflatten } from "flat";
import { encode, decode } from "@toon-format/toon";
import { projectForLlm } from "./llm-projection";

export function encodeLlmResult(value: Record<string, unknown>, maxBytes = 256 * 1024): string {
  assertFlattenable(value);
  let projected = projectForLlm(value);
  let encoded = encode(flatten(projected));
  while (Buffer.byteLength(encoded, "utf8") > maxBytes) {
    const next = projectForLlm(projected, Math.max(1, Math.floor(
      Math.max(1, countProjectedItems(projected)) / 2,
    )));
    if (JSON.stringify(next) === JSON.stringify(projected)) {
      throw new Error("LLM result exceeds the configured encoding budget");
    }
    projected = { ...next, truncated: true, completeness: "partial" };
    encoded = encode(flatten(projected));
  }
  return encoded;
}

function assertFlattenable(value: unknown, path = "result"): void {
  if (Array.isArray(value)) {
    value.forEach((item, index) => assertFlattenable(item, `${path}[${index}]`));
    return;
  }
  if (!value || typeof value !== "object") return;
  for (const [key, item] of Object.entries(value)) {
    if (key.includes(".") || key.includes("[")) {
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