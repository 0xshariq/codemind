import { encodeLlmResult } from "../results/llm-encoding";
import { validateCanonicalResult } from "../results/validate";
import { createRegisteredToolExecutor } from "../tools/executor";

export async function executeToolLoop(
  calls: readonly { name: string; input: unknown }[],
  budget = 8,
): Promise<unknown[]> {
  const executor = createRegisteredToolExecutor();
  const results: unknown[] = [];
  const seen = new Set<string>();
  for (const call of calls.slice(0, budget)) {
    const key = `${call.name}:${JSON.stringify(call.input)}`;
    if (seen.has(key)) throw new Error("agent tool loop stopped: duplicate tool call");
    seen.add(key);
    const result = await executor(call.name, call.input);
    validateCanonicalResult(result);
    results.push(
      result && typeof result === "object" && !Array.isArray(result)
        ? encodeLlmResult(result as Record<string, unknown>)
        : result,
    );
  }
  return results;
}