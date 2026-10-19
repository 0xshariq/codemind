import { analyzeChange } from "./analyze-change";
import type { ToolInput } from "../schemas";

export async function getImpact(input: ToolInput) {
  const result = await analyzeChange(input);
  const data = result.data && typeof result.data === "object" && !Array.isArray(result.data)
    ? result.data as Record<string, unknown> : {};
  return {
    ...result,
    data: {
      selectedEntityId: data.selectedEntityId ?? data.selected_entity_id ?? input.symbolId,
      impactedEntityIds: data.impactedEntityIds ?? data.impacted_entity_ids ?? [],
      paths: data.impactPaths ?? data.impact_paths ?? data.paths ?? [],
      impact: data.impact ?? null,
    },
  };
}
