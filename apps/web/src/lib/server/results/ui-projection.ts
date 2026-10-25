export type UiEvidenceReference = { path: string; line?: number; startLine?: number; endLine?: number; snapshotId: string };
export type UiResultData = {
  selected_entity_id?: string;
  impact_paths?: Array<{ entity_id: string; classification: string; depth: number; relationships?: Array<{ kind: string; from: string; to: string; direction?: string }> }>;
  test_candidates?: Array<{ test_id: string; name: string; path: string; evidence_path?: string; impact?: string }>;
  risk?: string;
  risk_assessment?: { rationale?: string[]; unknown_factors?: string[]; confidence?: number | string } | null;
  impact?: { impacted_entity_ids?: string[]; availability?: string; truncated?: boolean } | null;
  evidence_records?: Array<{ id: string; derivation: string; source?: UiEvidenceReference | null }>;
  recommendations?: Array<{ id: string; title: string; detail?: string; priority?: string; evidence?: string[] }>;
  relationships?: Array<{ kind: string; from: string; to: string; direction?: string }>;
  [key: string]: unknown;
};
export type UiCanonicalResult = {
  outcome: string;
  completeness: string;
  freshness: string;
  context?: { repositoryId: string; workspaceId: string; sourceSnapshotId: string; graphVersion: string };
  correlation?: { requestId: string; traceId: string; operationId: string };
  data: UiResultData;
  evidence?: UiEvidenceReference[];
  diagnostics?: Array<string | { code?: string; message: string; path?: string; severity?: string }>;
  limitations?: string[];
};

function record(value: unknown, name: string): Record<string, unknown> { if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error(`UI result ${name} must be an object`); return value as Record<string, unknown>; }
function requiredString(value: unknown, name: string) { if (typeof value !== "string" || !value.trim()) throw new Error(`UI result ${name} is invalid`); return value; }

/** Converts the canonical result into a bounded, browser-safe DTO without deriving new intelligence. */
export function projectForUi(value: unknown): UiCanonicalResult {
  const result = record(value, "must be a canonical result object");
  const outcome = requiredString(result.outcome, "outcome");
  const data = record(result.data, "data");
  const contextValue = result.context === undefined ? undefined : record(result.context, "context");
  const context = contextValue ? {
    repositoryId: requiredString(contextValue.repositoryId ?? contextValue.repository_id, "context.repositoryId"),
    workspaceId: requiredString(contextValue.workspaceId ?? contextValue.workspace_id, "context.workspaceId"),
    sourceSnapshotId: requiredString(contextValue.sourceSnapshotId ?? contextValue.source_snapshot_id, "context.sourceSnapshotId"),
    graphVersion: requiredString(contextValue.graphVersion ?? contextValue.graph_version, "context.graphVersion"),
  } : undefined;
  const correlationValue = result.correlation === undefined ? undefined : record(result.correlation, "correlation");
  const correlation = correlationValue ? {
    requestId: requiredString(correlationValue.requestId ?? correlationValue.request_id, "correlation.requestId"),
    traceId: requiredString(correlationValue.traceId ?? correlationValue.trace_id, "correlation.traceId"),
    operationId: requiredString(correlationValue.operationId ?? correlationValue.operation_id, "correlation.operationId"),
  } : undefined;
  const diagnostics = Array.isArray(result.diagnostics) ? result.diagnostics.slice(0, 100).flatMap((item): NonNullable<UiCanonicalResult["diagnostics"]> => { if (typeof item === "string" && item.trim()) return [item]; if (!item || typeof item !== "object" || Array.isArray(item)) return []; const diagnostic = item as Record<string, unknown>; if (typeof diagnostic.message !== "string" || !diagnostic.message.trim()) return []; return [{ code: typeof diagnostic.code === "string" ? diagnostic.code : undefined, message: diagnostic.message, path: typeof diagnostic.path === "string" ? diagnostic.path : undefined, severity: typeof diagnostic.severity === "string" ? diagnostic.severity : undefined }]; }) : undefined;
  const limitations = Array.isArray(result.limitations) ? result.limitations.slice(0, 100).filter((item): item is string => typeof item === "string" && item.trim().length > 0) : undefined;
  const evidence = Array.isArray(result.evidence) ? result.evidence.slice(0, 100).map((item) => { const source = record(item, "evidence"); return { path: requiredString(source.path, "evidence.path"), ...(source.line !== undefined ? { line: Number(source.line) } : {}), ...(source.startLine !== undefined || source.start_line !== undefined ? { startLine: Number(source.startLine ?? source.start_line) } : {}), ...(source.endLine !== undefined || source.end_line !== undefined ? { endLine: Number(source.endLine ?? source.end_line) } : {}), snapshotId: requiredString(source.snapshotId ?? source.snapshot_id, "evidence.snapshotId") }; }) : undefined;
  return { outcome, completeness: typeof result.completeness === "string" ? result.completeness : "unknown", freshness: typeof result.freshness === "string" ? result.freshness : "unknown", ...(context ? { context } : {}), ...(correlation ? { correlation } : {}), data: { ...data }, ...(evidence ? { evidence } : {}), ...(diagnostics ? { diagnostics } : {}), ...(limitations ? { limitations } : {}) };
}
