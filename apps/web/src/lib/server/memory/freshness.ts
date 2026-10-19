import "server-only";

export type Freshness = "fresh" | "stale" | "unknown";

export function classifyFreshness(
  memorySnapshot: string,
  currentSnapshot: string,
  lifecycle: string,
  memoryGraphVersion?: string,
  currentGraphVersion?: string,
  provenanceCompatible = true,
): Freshness {
  if (lifecycle !== "validated") return "unknown";
  if (!memorySnapshot || !currentSnapshot || !memoryGraphVersion || !currentGraphVersion) return "unknown";
  if (!provenanceCompatible) return "stale";
  return memorySnapshot === currentSnapshot && memoryGraphVersion === currentGraphVersion ? "fresh" : "stale";
}
