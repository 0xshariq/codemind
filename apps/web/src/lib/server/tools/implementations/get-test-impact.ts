import { analyzeChange } from "./analyze-change";
export async function getTestImpact(input: Parameters<typeof analyzeChange>[0]) {
  const result = await analyzeChange(input);
  return { ...result, data: result.data && typeof result.data === "object" ? { testCandidates: (result.data as { testCandidates?: unknown }).testCandidates } : result.data };
}