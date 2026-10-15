import type { CanonicalJsonValue } from "./canonical-json";

export function validateCanonicalResult(value: unknown): asserts value is {
  outcome: string;
  data: CanonicalJsonValue;
} {
  if (!value || typeof value !== "object") {
    throw new Error("Canonical result must be an object");
  }
  const result = value as Record<string, unknown>;
  if (typeof result.outcome !== "string" || !("data" in result)) {
    throw new Error("Canonical result requires outcome and data");
  }
}