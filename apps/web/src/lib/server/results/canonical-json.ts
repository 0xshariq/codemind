export type CanonicalJsonValue =
  | null
  | boolean
  | number
  | string
  | CanonicalJsonValue[]
  | { [key: string]: CanonicalJsonValue };

export function parseCanonicalJson(value: string): CanonicalJsonValue {
  if (Buffer.byteLength(value, "utf8") > 256 * 1024) throw new Error("Canonical JSON exceeds the result byte limit");
  const parsed: unknown = JSON.parse(value);
  validateJsonValue(parsed, 0);
  return parsed as CanonicalJsonValue;
}

function validateJsonValue(value: unknown, depth: number): void {
  if (depth > 64) throw new Error("Canonical JSON exceeds the nesting limit");
  if (value === null || typeof value === "string" || typeof value === "boolean") return;
  if (typeof value === "number") {
    if (!Number.isFinite(value)) throw new Error("Canonical JSON contains a non-finite number");
    return;
  }
  if (Array.isArray(value)) {
    for (const child of value) validateJsonValue(child, depth + 1);
    return;
  }
  if (typeof value === "object") {
    for (const child of Object.values(value)) validateJsonValue(child, depth + 1);
    return;
  }
  throw new Error("Canonical JSON contains an unsupported value");
}
