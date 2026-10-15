import type { ToolInput } from "../schemas";

export function getSymbol(input: ToolInput) {
  return input.symbolId
    ? { outcome: "unsupported", data: { symbolId: input.symbolId }, limitations: ["Symbol detail query is not exposed by the Day 1 Rust contract."] }
    : { outcome: "invalid", data: null, limitations: ["symbolId is required"] };
}