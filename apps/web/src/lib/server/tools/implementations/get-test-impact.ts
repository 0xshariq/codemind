import { analyzeChange } from "./analyze-change";
import type { ToolInput } from "../schemas";

export async function getTestImpact(input: ToolInput) {
  const result = await analyzeChange(input);
  const data = result.data && typeof result.data === "object" && !Array.isArray(result.data)
    ? result.data as Record<string, unknown> : {};
  const candidates = data.testCandidates ?? data.test_candidates;
  const tests = Array.isArray(candidates) ? candidates : [];
  const seen = new Set<string>();
  const deduplicated = tests.filter((item) => {
    if (!item || typeof item !== "object") return false;
    const candidate = item as Record<string, unknown>;
    const id = String(candidate.testId ?? candidate.test_id ?? "");
    if (!id || seen.has(id)) return false;
    seen.add(id);
    return true;
  }).slice(0, input.limit ?? 20);
  return {
    ...result,
    data: { tests: deduplicated },
  };
}
