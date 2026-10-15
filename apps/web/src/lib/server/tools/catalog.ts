import { toolInputSchema } from "./schemas";

export const APPROVED_TOOL_NAMES = [
  "get_repository_context",
  "search_code",
  "get_symbol",
  "query_code_relationships",
  "get_historical_changes",
  "get_test_intelligence",
  "analyze_change",
  "get_impact",
  "get_risk",
  "get_test_impact",
  "get_evidence",
] as const;

export type ToolName = (typeof APPROVED_TOOL_NAMES)[number];

export type ToolDefinition = {
  name: ToolName;
  description: string;
  input: typeof toolInputSchema;
  timeoutMs: number;
  maxCalls: number;
};

export const TOOL_CATALOG: readonly ToolDefinition[] = APPROVED_TOOL_NAMES.map((name) => ({
  name,
  description: `Grounded CodeMind ${name.replaceAll("_", " ")} query`,
  input: toolInputSchema,
  timeoutMs: 5_000,
  maxCalls: 3,
}));