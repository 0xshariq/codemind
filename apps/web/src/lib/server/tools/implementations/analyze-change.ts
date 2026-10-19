import { analyzeChange as analyzeChangeRpc } from "../../rust-client/client";
import type { ToolInput } from "../schemas";
import { canonicalToolResult } from "./canonical";

export async function analyzeChange(input: ToolInput) {
  if (!input.symbolId) throw new Error("symbolId is required");
  const response = await analyzeChangeRpc({
    symbolId: input.symbolId,
    maxDepth: input.maxDepth ?? 3,
    context: input.context,
  });
  return canonicalToolResult(response, input);
}
