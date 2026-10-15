import { analyzeChange } from "./analyze-change";
export async function getEvidence(input: Parameters<typeof analyzeChange>[0]) {
  const result = await analyzeChange(input);
  return { ...result, data: result.data && typeof result.data === "object" ? { evidence: (result.data as { evidence?: unknown }).evidence } : result.data };
}