import { z } from "zod";

export const contextSchema = z.object({
  workspaceId: z.string().trim().min(1).max(1_024),
  repositoryId: z.string().trim().min(1).max(1_024),
  sourceSnapshotId: z.string().trim().min(1).max(1_024),
  graphVersion: z.string().trim().min(1).max(1_024),
}).strict();

const base = { context: contextSchema };
const symbol = { symbolId: z.string().trim().min(1).max(1_024) };
const query = { query: z.string().trim().min(1).max(500) };
const limit = { limit: z.number().int().min(1).max(100).default(20) };

export const toolInputSchemas = {
  get_repository_context: z.object(base).strict(),
  search_code: z.object({ ...base, ...query, ...limit }).strict(),
  get_symbol: z.object({ ...base, ...symbol }).strict(),
  query_code_relationships: z.object({ ...base, ...symbol, ...limit }).strict(),
  get_historical_changes: z.object({ ...base, ...limit }).strict(),
  get_test_intelligence: z.object({ ...base, ...symbol, ...limit }).strict(),
  analyze_change: z.object({ ...base, ...symbol, maxDepth: z.number().int().min(1).max(8).default(3) }).strict(),
  get_impact: z.object({ ...base, ...symbol, maxDepth: z.number().int().min(1).max(8).default(3) }).strict(),
  get_risk: z.object({ ...base, ...symbol, maxDepth: z.number().int().min(1).max(8).default(3) }).strict(),
  get_test_impact: z.object({ ...base, ...symbol, ...limit }).strict(),
  get_evidence: z.object({ ...base, ...symbol, ...limit }).strict(),
} as const;

export type ToolName = keyof typeof toolInputSchemas;
export type ToolInput = {
  context: z.infer<typeof contextSchema>;
  symbolId?: string;
  query?: string;
  limit?: number;
  maxDepth?: number;
};

export function parseToolInput(name: string, value: unknown): ToolInput {
  if (!(name in toolInputSchemas)) throw new Error(`unsupported semantic tool: ${name}`);
  return toolInputSchemas[name as ToolName].parse(value) as ToolInput;
}

// Authorization needs a bounded context before the tool name is dispatched.
export const toolInputSchema = z.object({
  ...base,
  symbolId: symbol.symbolId.optional(),
  query: query.query.optional(),
  limit: limit.limit,
  maxDepth: z.number().int().min(1).max(8).optional(),
}).strict();
