import { querySymbolRelationships } from "../../rust-client/client";
import type { ToolInput } from "../schemas";

export function queryCodeRelationships(input: ToolInput) {
  if (!input.symbolId) throw new Error("symbolId is required");
  return querySymbolRelationships({
    symbolId: input.symbolId,
    limit: input.limit,
    context: input.context,
  }).then((response) => {
    const result = response.result;
    if (!result || typeof result !== "object") throw new Error("Rust returned an invalid relationship result");
    const canonical = result as {
      outcome?: number;
      completeness?: number;
      freshness?: number;
      context?: ToolInput["context"];
      dataJson?: string;
      limitations?: string[];
    };
    if (typeof canonical.dataJson !== "string") {
      throw new Error("Rust returned no canonical relationship data");
    }
    return {
      outcome: canonical.outcome === 2 ? "empty" : "success",
      completeness: canonical.completeness === 3 ? "truncated" : "complete",
      freshness: canonical.freshness === 2 ? "stale" : "current",
      context: canonical.context ?? input.context,
      data: JSON.parse(canonical.dataJson) as unknown,
      limitations: canonical.limitations ?? [],
    };
  });
}