import { analyzeChange } from "./analyze-change";
import type { ToolInput } from "../schemas";

export async function getEvidence(input: ToolInput) {
  const result = await analyzeChange(input);
  const data = result.data && typeof result.data === "object" && !Array.isArray(result.data)
    ? result.data as Record<string, unknown> : {};
  const records = data.evidenceRecords ?? data.evidence_records ?? [];
  const evidence = Array.isArray(result.evidence) ? result.evidence : [];
  const missing = evidence.length === 0 && (!Array.isArray(records) || records.length === 0);
  return {
    ...result,
    data: { references: evidence, records: Array.isArray(records) ? records.slice(0, input.limit ?? 20) : [] },
    limitations: missing
      ? [...(Array.isArray(result.limitations) ? result.limitations : []), "ChIE did not return source evidence for this result."]
      : result.limitations,
  };
}
