import { encodeLlmResult } from "../results/llm-encoding";
import { validateCanonicalResult } from "../results/validate";
import { createRegisteredToolExecutor } from "../tools/executor";

export type ToolCallAuthorizer = (name: string, input: unknown) => Promise<void>;

export async function executeToolLoop(
  calls: readonly { name: string; input: unknown }[],
  budget = 8,
  onToolStart?: (name: string) => void,
  onCanonicalResult?: (result: unknown) => void,
  authorize?: ToolCallAuthorizer,
): Promise<unknown[]> {
  const executor = createRegisteredToolExecutor();
  const results: unknown[] = [];
  const seen = new Set<string>();
  for (const call of calls.slice(0, budget)) {
    onToolStart?.(call.name);
    const key = `${call.name}:${JSON.stringify(call.input)}`;
    if (seen.has(key)) throw new Error("agent tool loop stopped: duplicate tool call");
    seen.add(key);
    if (!authorize) throw new Error("agent tool execution requires authenticated repository authorization");
    await authorize(call.name, call.input);
    let result: unknown;
    try {
      result = await executor(call.name, call.input);
    } catch (error) {
      // A single semantic query must not discard graph evidence already
      // gathered for this investigation. Authorization remains outside this
      // boundary and still fails closed; only an authorized tool execution is
      // downgraded to an explicit unavailable result.
      const input = call.input && typeof call.input === "object" && !Array.isArray(call.input)
        ? call.input as Record<string, unknown> : {};
      const context = input.context && typeof input.context === "object" && !Array.isArray(input.context)
        ? input.context : undefined;
      result = {
        outcome: "unavailable",
        completeness: "incomplete",
        freshness: "unknown",
        ...(context ? { context } : {}),
        data: null,
        evidence: [],
        diagnostics: [],
        limitations: [`The ${call.name} semantic tool could not return evidence; other available evidence was retained.`],
      };
      // Keep details in the server log for diagnosis without exposing internal
      // errors or repository data to the model or browser.
      console.error("CodeMind semantic tool failed", {
        tool: call.name,
        message: error instanceof Error ? error.message : "unknown error",
      });
    }
    validateCanonicalResult(result);
    onCanonicalResult?.(result);
    try {
      results.push(encodeLlmResult(result as Record<string, unknown>));
    } catch {
      throw new Error(`semantic tool result could not be safely encoded: ${call.name}`);
    }
  }
  if (calls.length > budget) {
    results.push(encodeLlmResult({ outcome: "partial", data: null, completeness: "truncated", limitations: ["Tool-call budget exhausted before all requested calls were executed."] }));
  }
  return results;
}
