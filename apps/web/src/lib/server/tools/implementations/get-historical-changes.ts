import type { ToolInput } from "../schemas";

export function getHistoricalChanges(input: ToolInput) {
  return {
    outcome: "unsupported",
    completeness: "incomplete",
    freshness: "unknown",
    context: input.context,
    data: { changes: [] },
    evidence: [],
    diagnostics: [],
    limitations: ["The Rust contract does not expose persisted change history."],
  };
}
