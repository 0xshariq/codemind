import { querySymbolRelationships } from "../../rust-client/client";
import type { ToolInput } from "../schemas";
import { canonicalToolResult } from "./canonical";

export async function queryCodeRelationships(input: ToolInput) {
  if (!input.symbolId) throw new Error("symbolId is required");
  const response = await querySymbolRelationships({
    symbolId: input.symbolId,
    limit: input.limit ?? 20,
    context: input.context,
  });
  return canonicalToolResult(response, input);
}
