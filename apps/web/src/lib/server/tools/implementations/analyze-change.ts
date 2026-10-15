import { analyzeChange as analyzeChangeRpc } from "../../rust-client/client";
import type { ToolInput } from "../schemas";

export async function analyzeChange(input: ToolInput) {
  if (!input.symbolId) throw new Error("symbolId is required");
  const response = await analyzeChangeRpc({ symbolId: input.symbolId, maxDepth: 4, context: input.context });
  const result = response.result;
  if (!result || typeof result !== "object") throw new Error("Rust returned an invalid ChIE result");
  const canonical = result as { dataJson?: string; outcome?: number; completeness?: number; freshness?: number; context?: ToolInput["context"]; limitations?: string[] };
  if (!canonical.dataJson) throw new Error("Rust returned no ChIE analysis");
  return {
    outcome: canonical.outcome === 2 ? "empty" : "success",
    completeness: canonical.completeness === 3 ? "truncated" : canonical.completeness === 2 ? "partial" : "complete",
    freshness: canonical.freshness === 2 ? "stale" : "current",
    context: canonical.context ?? input.context,
    data: JSON.parse(canonical.dataJson) as unknown,
    limitations: canonical.limitations ?? [],
  };
}