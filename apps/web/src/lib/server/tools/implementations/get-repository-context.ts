import type { ToolInput } from "../schemas";

export function getRepositoryContext(input: ToolInput) {
  return { outcome: "success", data: input.context, context: input.context };
}