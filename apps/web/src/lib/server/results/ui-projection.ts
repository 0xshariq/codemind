export type UiEvidenceReference = {
  path: string;
  line?: number;
  startLine?: number;
  endLine?: number;
  snapshotId: string;
};

export type UiCanonicalResult = {
  outcome: string;
  completeness?: string;
  freshness?: string;
  context?: { repositoryId: string; workspaceId: string; sourceSnapshotId: string; graphVersion: string };
  correlation?: { requestId: string; traceId: string; operationId: string };
  data: unknown;
  evidence?: UiEvidenceReference[];
  diagnostics?: Array<string | { code?: string; message: string; path?: string; severity?: string }>;
  limitations?: string[];
};

/** UI DTO boundary: preserve canonical semantic fields and avoid protobuf enum objects. */
export function projectForUi(value: unknown): UiCanonicalResult {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("UI result must be a canonical result object");
  const result = value as Record<string, unknown>;
  if (typeof result.outcome !== "string" || !("data" in result)) throw new Error("UI result is missing canonical outcome or data");
  return {
    outcome: result.outcome,
    ...(typeof result.completeness === "string" ? { completeness: result.completeness } : {}),
    ...(typeof result.freshness === "string" ? { freshness: result.freshness } : {}),
    ...(result.context && typeof result.context === "object" ? { context: result.context as UiCanonicalResult["context"] } : {}),
    ...(result.correlation && typeof result.correlation === "object" ? { correlation: result.correlation as UiCanonicalResult["correlation"] } : {}),
    data: result.data,
    ...(Array.isArray(result.evidence) ? { evidence: result.evidence as UiEvidenceReference[] } : {}),
    ...(Array.isArray(result.diagnostics) ? { diagnostics: result.diagnostics.flatMap<NonNullable<UiCanonicalResult["diagnostics"]>[number]>((item) => {
      if (typeof item === "string") return [item];
      if (!item || typeof item !== "object" || Array.isArray(item)) return [];
      const value = item as Record<string, unknown>;
      if (typeof value.message !== "string") return [];
      return [{
        code: typeof value.code === "string" ? value.code : undefined,
        message: value.message,
        path: typeof value.path === "string" ? value.path : undefined,
        severity: typeof value.severity === "string" ? value.severity : undefined,
      }];
    }) } : {}),
    ...(Array.isArray(result.limitations) ? { limitations: result.limitations.filter((item): item is string => typeof item === "string") } : {}),
  };
}
