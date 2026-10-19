import { analyzeChange } from "./analyze-change";
import type { ToolInput } from "../schemas";

export async function getTestIntelligence(input: ToolInput) {
  const result = await analyzeChange(input);
  const data = result.data && typeof result.data === "object" && !Array.isArray(result.data)
    ? result.data as Record<string, unknown> : {};
  return {
    ...result,
    data: {
      source: "chie_test_evidence",
      tests: data.testCandidates ?? data.test_candidates ?? [],
      limitations: ["Test facts are exposed through the combined ChIE result; this is not an independent TIE query."],
    },
    limitations: [...(Array.isArray(result.limitations) ? result.limitations : []), "Test facts are exposed through the combined ChIE result; this is not an independent TIE query."],
  };
}
