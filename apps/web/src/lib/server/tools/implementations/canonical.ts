import { parseCanonicalJson } from "../../results/canonical-json";
import { validateCanonicalResult } from "../../results/validate";
import type { ToolInput } from "../schemas";

const outcomes = ["invalid", "success", "empty", "partial", "unsupported", "invalid", "unavailable", "failed"] as const;
const completeness = ["incomplete", "complete", "incomplete", "truncated"] as const;
const freshness = ["unknown", "current", "stale"] as const;
const camelContext = (value: unknown, fallback: ToolInput["context"]) => {
  const context = value && typeof value === "object" ? value as Record<string, unknown> : {};
  return {
    repositoryId: String(context.repositoryId ?? context.repository_id ?? fallback.repositoryId),
    workspaceId: String(context.workspaceId ?? context.workspace_id ?? fallback.workspaceId),
    sourceSnapshotId: String(context.sourceSnapshotId ?? context.source_snapshot_id ?? fallback.sourceSnapshotId),
    graphVersion: String(context.graphVersion ?? context.graph_version ?? fallback.graphVersion),
  };
};

export function canonicalToolResult(response: Record<string, unknown>, input: ToolInput) {
  // proto-loader exposes oneof members directly (`data` or `error`) unless its
  // `oneofs` option is enabled. Accept that generated shape and the wrapped
  // shape used by deterministic contract fixtures.
  const raw = response.result ?? (response.data ? { data: response.data } : response.error ? { error: response.error } : undefined);
  if (!raw || typeof raw !== "object") throw new Error("Rust returned no canonical result");
  const envelope = raw as Record<string, unknown>;
  if (envelope.error && typeof envelope.error === "object") {
    const error = envelope.error as Record<string, unknown>;
    const context = camelContext(error.context, input.context);
    if (Object.entries(input.context).some(([key, value]) => context[key as keyof typeof context] !== value)) {
      throw new Error("Rust error context does not match the authorized repository snapshot");
    }
    return {
      outcome: error.retryable ? "unavailable" : "failed",
      completeness: "incomplete",
      freshness: "unknown",
      context,
      correlation: error.correlation,
      data: null,
      evidence: [],
      diagnostics: [String(error.message ?? "Rust operation failed")],
      limitations: [],
    };
  }
  const result = envelope.data && typeof envelope.data === "object"
    ? envelope.data as Record<string, unknown>
    : envelope;
  const rawData = result.dataJson ?? result.data_json;
  if (typeof rawData !== "string") throw new Error("Rust returned no canonical result data");
  const number = (value: unknown) => typeof value === "number" ? value : Number(value);
  const refs = Array.isArray(result.evidence) ? result.evidence.map((item) => {
    const ref = item as Record<string, unknown>;
    return {
      path: String(ref.path ?? ""),
      startLine: Number(ref.startLine ?? ref.start_line ?? ref.line ?? 0),
      endLine: Number(ref.endLine ?? ref.end_line ?? ref.line ?? 0),
      snapshotId: String(ref.snapshotId ?? ref.snapshot_id ?? ""),
    };
  }) : [];
  const value = {
    outcome: outcomes[number(result.outcome)] ?? "failed",
    completeness: completeness[number(result.completeness)] ?? "incomplete",
    freshness: freshness[number(result.freshness)] ?? "unknown",
    context: camelContext(result.context, input.context),
    correlation: result.correlation,
    data: parseCanonicalJson(rawData),
    evidence: refs,
    diagnostics: Array.isArray(result.diagnostics) ? result.diagnostics.map((item) => {
      if (typeof item === "string") return item;
      const diagnostic = item && typeof item === "object" ? item as Record<string, unknown> : {};
      return {
        code: String(diagnostic.code ?? "unknown"),
        message: String(diagnostic.message ?? "diagnostic"),
        path: String(diagnostic.path ?? ""),
        severity: String(diagnostic.severity ?? "unknown"),
      };
    }) : [],
    limitations: Array.isArray(result.limitations) ? result.limitations : [],
  };
  const canonicalContext = value.context;
  if (Object.entries(input.context).some(([key, expected]) => canonicalContext[key as keyof typeof canonicalContext] !== expected)) {
    throw new Error("Rust canonical result context does not match the request");
  }
  validateCanonicalResult(value);
  return value;
}
