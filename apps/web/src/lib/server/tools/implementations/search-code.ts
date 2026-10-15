import type { ToolInput } from "../schemas";

export function searchCode(input: ToolInput) {
  return { outcome: "unsupported", data: { query: input.query ?? "" }, limitations: ["Repository search is not exposed by the current Rust contract."] };
}