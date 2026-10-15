import type { ToolInput } from "../schemas";
import { analyzeChange } from "./analyze-change";
import { getEvidence } from "./get-evidence";
import { getHistoricalChanges } from "./get-historical-changes";
import { getImpact } from "./get-impact";
import { getRepositoryContext } from "./get-repository-context";
import { getRisk } from "./get-risk";
import { getSymbol } from "./get-symbol";
import { getTestImpact } from "./get-test-impact";
import { getTestIntelligence } from "./get-test-intelligence";
import { queryCodeRelationships } from "./query-code-relationships";
import { searchCode } from "./search-code";

export const TOOL_HANDLERS: Record<string, (input: ToolInput) => unknown> = {
  get_repository_context: getRepositoryContext,
  search_code: searchCode,
  get_symbol: getSymbol,
  query_code_relationships: queryCodeRelationships,
  get_historical_changes: getHistoricalChanges,
  get_test_intelligence: getTestIntelligence,
  analyze_change: analyzeChange,
  get_impact: getImpact,
  get_risk: getRisk,
  get_test_impact: getTestImpact,
  get_evidence: getEvidence,
};
