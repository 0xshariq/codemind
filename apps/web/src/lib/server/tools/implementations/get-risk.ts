import { analyzeChange } from "./analyze-change";
import type { ToolInput } from "../schemas";

export async function getRisk(input: ToolInput) {
  const result = await analyzeChange(input);
  const data = result.data && typeof result.data === "object" && !Array.isArray(result.data)
    ? result.data as Record<string, unknown> : {};
  return {
    ...result,
    data: {
      level: data.risk ?? data.risk_level ?? "unknown",
      assessment: data.riskAssessment ?? data.risk_assessment ?? null,
      unknownFactors: data.unknown_factors ?? [],
    },
  };
}
