const IDENTITY_FIELDS = new Set(["id", "entityId", "entity_id", "symbolId", "symbol_id", "repositoryId", "repository_id", "sourceSnapshotId", "source_snapshot_id", "graphVersion", "graph_version", "path", "relationship", "relationshipType", "relationship_type", "direction", "kind", "outcome", "completeness", "freshness"]);

/** Keep canonical identity/provenance while bounding nested result data for the model. */
export function projectForLlm(value: Record<string, unknown>, maxItems = 20) {
  const limit = Number.isSafeInteger(maxItems) ? Math.max(1, Math.min(100, maxItems)) : 20;
  let truncated = false;
  const visit = (item: unknown, depth: number): unknown => {
    if (depth > 32) { truncated = true; return "[depth limit]"; }
    if (Array.isArray(item)) { const selected = item.slice(0, limit).map((child) => visit(child, depth + 1)); if (item.length > limit) truncated = true; return selected; }
    if (!item || typeof item !== "object") return item;
    const projected: Record<string, unknown> = {};
    for (const [key, child] of Object.entries(item as Record<string, unknown>)) {
      if (["rawBytes", "stack", "debug", "authorization"].includes(key)) continue;
      projected[key] = IDENTITY_FIELDS.has(key) || child === null || typeof child !== "object" ? child : visit(child, depth + 1);
    }
    return projected;
  };
  const projection = visit(value, 0) as Record<string, unknown>;
  if (truncated) { projection.truncated = true; projection.completeness = "partial"; if (projection.outcome === "success") projection.outcome = "partial"; }
  return projection;
}
