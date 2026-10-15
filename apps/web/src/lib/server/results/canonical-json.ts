export type CanonicalJsonValue =
  | null
  | boolean
  | number
  | string
  | CanonicalJsonValue[]
  | { [key: string]: CanonicalJsonValue };

export function parseCanonicalJson(value: string): CanonicalJsonValue {
  const parsed: unknown = JSON.parse(value);
  if (parsed === null || typeof parsed !== "object") {
    throw new Error("Canonical result data must be a JSON object or array");
  }
  return parsed as CanonicalJsonValue;
}