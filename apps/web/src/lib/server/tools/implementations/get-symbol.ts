import type { ToolInput } from "../schemas";

export function getSymbol(input: ToolInput) {
  return {
    outcome: "unsupported",
    completeness: "incomplete",
    freshness: "unknown",
    context: input.context,
    data: { symbolId: input.symbolId },
    evidence: [],
    diagnostics: [],
    limitations: ["The Rust intelligence contract does not expose a standalone symbol-detail query."],
  };
}
