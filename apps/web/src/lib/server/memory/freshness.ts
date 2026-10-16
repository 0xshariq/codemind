import "server-only";

export type Freshness = "fresh" | "stale" | "unknown";

export function classifyFreshness(
  memorySnapshot: string,
  currentSnapshot: string,
  lifecycle: string,
): Freshness {
  if (lifecycle !== "validated") return "unknown";
  if (!memorySnapshot || !currentSnapshot) return "unknown";
  return memorySnapshot === currentSnapshot ? "fresh" : "stale";
}