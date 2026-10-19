import { toolInputSchemas, type ToolName } from "./schemas";

export const APPROVED_TOOL_NAMES = [
  "get_repository_context", "search_code", "get_symbol", "query_code_relationships",
  "get_historical_changes", "get_test_intelligence", "analyze_change", "get_impact",
  "get_risk", "get_test_impact", "get_evidence",
] as const satisfies readonly ToolName[];

export type { ToolName };
export type ToolDefinition = {
  name: ToolName;
  version: 1;
  description: string;
  authorization: "repository_snapshot";
  rustMapping: string | null;
  outputContract: "canonical_result";
  timeoutMs?: number;
  maxCalls: number;
  input: (typeof toolInputSchemas)[ToolName];
  unsupportedBehavior: "explicit_unsupported";
};

const descriptions: Record<ToolName, { description: string; rustMapping: string | null }> = {
  get_repository_context: { description: "Resolve and report the authoritative repository snapshot and graph version.", rustMapping: "RepositoryService.ResolveRepository" },
  search_code: { description: "Search canonical graph entity names and paths in the authorized repository snapshot.", rustMapping: "IntelligenceService.SearchCode" },
  get_symbol: { description: "Resolve a canonical code symbol by its graph entity ID.", rustMapping: null },
  query_code_relationships: { description: "Query bounded, typed relationships connected to one canonical symbol.", rustMapping: "IntelligenceService.QuerySymbolRelationships" },
  get_historical_changes: { description: "Retrieve persisted historical change facts when available.", rustMapping: null },
  get_test_intelligence: { description: "Return test evidence associated with the selected symbol from supported analysis facts.", rustMapping: "IntelligenceService.AnalyzeChange (TIE-derived evidence)" },
  analyze_change: { description: "Run bounded ChIE change-impact analysis for a canonical symbol.", rustMapping: "IntelligenceService.AnalyzeChange" },
  get_impact: { description: "Return the impact paths and affected entities from ChIE analysis.", rustMapping: "IntelligenceService.AnalyzeChange" },
  get_risk: { description: "Return evidence-backed risk assessment from ChIE analysis.", rustMapping: "IntelligenceService.AnalyzeChange" },
  get_test_impact: { description: "Return deduplicated tests impacted by the selected change.", rustMapping: "IntelligenceService.AnalyzeChange (TIE-derived evidence)" },
  get_evidence: { description: "Return source evidence references attached to canonical change analysis.", rustMapping: "IntelligenceService.AnalyzeChange" },
};

export const TOOL_CATALOG: readonly ToolDefinition[] = APPROVED_TOOL_NAMES.map((name) => ({
  name,
  version: 1,
  ...descriptions[name],
  authorization: "repository_snapshot",
  outputContract: "canonical_result",
  maxCalls: 3,
  input: toolInputSchemas[name],
  unsupportedBehavior: "explicit_unsupported",
}));
