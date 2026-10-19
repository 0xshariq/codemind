import { searchCode as searchCodeRpc } from "../../rust-client/client";
import type { ToolInput } from "../schemas";
import { canonicalToolResult } from "./canonical";

export async function searchCode(input: ToolInput) {
  if (!input.query) throw new Error("query is required");
  const response = await searchCodeRpc({
    query: input.query,
    limit: input.limit ?? 20,
    context: input.context,
  });
  return canonicalToolResult(response, input);
}
