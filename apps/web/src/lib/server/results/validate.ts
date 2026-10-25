import type { CanonicalJsonValue } from "./canonical-json";

export function validateCanonicalResult(value: unknown): asserts value is {
  outcome: "success" | "empty" | "partial" | "unsupported" | "unavailable" | "failed" | "invalid";
  data: CanonicalJsonValue;
} {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("Canonical result must be an object");
  }
  const result = value as Record<string, unknown>;
  const outcomes = ["success", "empty", "partial", "unsupported", "unavailable", "failed", "invalid"];
  const completeness = ["complete", "truncated", "incomplete", "partial"];
  const freshness = ["current", "stale", "unknown"];
  const get = (record: Record<string, unknown>, camel: string, snake: string) => record[camel] ?? record[snake];
  if (!outcomes.includes(String(result.outcome)) || !("data" in result)) throw new Error("Canonical result requires a supported outcome and data");
  if (Buffer.byteLength(JSON.stringify(result.data), "utf8") > 192 * 1024) throw new Error("Canonical result data exceeds its serialized byte limit");
  if (result.completeness !== undefined && !completeness.includes(String(result.completeness))) throw new Error("Canonical result completeness is invalid");
  if (result.freshness !== undefined && !freshness.includes(String(result.freshness))) throw new Error("Canonical result freshness is invalid");
  const context = get(result, "context", "context");
  let contextSnapshotId: string | undefined;
  if (context !== undefined) {
    if (!context || typeof context !== "object" || Array.isArray(context)) throw new Error("Canonical result context is invalid");
    const item = context as Record<string, unknown>;
    for (const [camel, snake] of [["repositoryId", "repository_id"], ["workspaceId", "workspace_id"], ["sourceSnapshotId", "source_snapshot_id"], ["graphVersion", "graph_version"]]) {
      if (typeof get(item, camel, snake) !== "string" || !String(get(item, camel, snake)).trim()) throw new Error(`Canonical result context is missing ${snake}`);
    }
    contextSnapshotId = String(get(item, "sourceSnapshotId", "source_snapshot_id"));
  }
  const correlation = result.correlation;
  if (correlation !== undefined) {
    if (!correlation || typeof correlation !== "object" || Array.isArray(correlation)) throw new Error("Canonical result correlation is invalid");
    const item = correlation as Record<string, unknown>;
    for (const [camel, snake] of [["requestId", "request_id"], ["traceId", "trace_id"], ["operationId", "operation_id"]]) {
      if (typeof get(item, camel, snake) !== "string" || !String(get(item, camel, snake)).trim()) throw new Error(`Canonical result correlation is missing ${snake}`);
    }
  }
  for (const key of ["evidence", "diagnostics", "limitations"]) {
    if (result[key] !== undefined && (!Array.isArray(result[key]) || result[key].length > 100)) throw new Error(`Canonical result ${key} is invalid or exceeds its limit`);
  }
  if (Array.isArray(result.evidence)) for (const reference of result.evidence) {
    if (!reference || typeof reference !== "object" || Array.isArray(reference)) throw new Error("Canonical evidence reference is invalid");
    const item = reference as Record<string, unknown>;
    const path = item.path;
    const snapshot = get(item, "snapshotId", "snapshot_id");
    const start = get(item, "startLine", "start_line") ?? item.line;
    const end = get(item, "endLine", "end_line") ?? item.line;
    if (typeof path !== "string" || !path.trim() || path.startsWith("/") || path.split(/[\\/]/).includes("..")
      || typeof snapshot !== "string" || !snapshot.trim() || (contextSnapshotId !== undefined && snapshot !== contextSnapshotId)
      || !Number.isSafeInteger(start) || !Number.isSafeInteger(end)
      || Number(start) < 1 || Number(end) < Number(start)) throw new Error("Canonical evidence reference is invalid or outside its source snapshot");
  }
  if (Array.isArray(result.diagnostics)) for (const diagnostic of result.diagnostics) {
    if (typeof diagnostic === "string" && diagnostic.trim()) continue;
    const message = diagnostic && typeof diagnostic === "object" && !Array.isArray(diagnostic)
      ? (diagnostic as Record<string, unknown>).message : undefined;
    if (typeof message !== "string" || !message.trim()) throw new Error("Canonical diagnostic is invalid");
  }
  if (Array.isArray(result.limitations) && result.limitations.some((item) => typeof item !== "string" || !item.trim())) {
    throw new Error("Canonical limitation is invalid");
  }
  if (result.completeness === "truncated" && result.outcome !== "partial") throw new Error("Truncated canonical results must be partial");
  if (result.outcome === "partial" && result.completeness === undefined) throw new Error("Partial canonical results must declare completeness");
  if (result.completeness && result.completeness !== "complete" && result.outcome === "success") {
    throw new Error("Incomplete canonical results cannot use success outcome");
  }
  if (Buffer.byteLength(JSON.stringify(result), "utf8") > 256 * 1024) throw new Error("Canonical result exceeds its serialized byte limit");
}
