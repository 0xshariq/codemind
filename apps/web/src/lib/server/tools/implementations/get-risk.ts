import { analyzeChange } from "./analyze-change";
export async function getRisk(input: Parameters<typeof analyzeChange>[0]) {
  const result = await analyzeChange(input);
  return { ...result, data: result.data && typeof result.data === "object" ? { risk: (result.data as { risk?: unknown }).risk } : result.data };
}