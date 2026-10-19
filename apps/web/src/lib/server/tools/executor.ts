import { parseToolInput, type ToolInput, type ToolName } from "./schemas";
import { accountToolResult, checkGuardrails, createGuardrailState, type GuardrailState } from "./guardrails";
import { getTool } from "./registry";
import { TOOL_HANDLERS } from "./implementations";
import { validateCanonicalResult } from "../results/validate";

export type ToolExecutor = (name: string, input: unknown, state?: GuardrailState) => Promise<unknown>;

export function createToolExecutor(handler: ToolExecutor): ToolExecutor {
  const state = createGuardrailState();
  return async (name, input, sharedState = state) => {
    const tool = getTool(name);
    const parsed: ToolInput = parseToolInput(tool.name, input);
    const contextKey = [parsed.context.workspaceId, parsed.context.repositoryId, parsed.context.sourceSnapshotId, parsed.context.graphVersion].join("\0");
    checkGuardrails(sharedState, `${tool.name}:${JSON.stringify(parsed)}`, tool.name, tool.maxCalls, contextKey);
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
      const execution = Promise.resolve(handler(tool.name, parsed, sharedState));
      const result = tool.timeoutMs === undefined
        ? await execution
        : await Promise.race([
          execution,
          new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new Error(`semantic tool timed out: ${tool.name}`)), tool.timeoutMs); }),
        ]);
      validateCanonicalResult(result);
      const serialized = JSON.stringify(result);
      if (serialized === undefined) throw new Error("semantic tool returned a non-serializable result");
      accountToolResult(sharedState, serialized);
      return result;
    } finally {
      if (timer) clearTimeout(timer);
    }
  };
}

export function createRegisteredToolExecutor(): ToolExecutor {
  return createToolExecutor(async (name, input) => {
    const toolHandler = TOOL_HANDLERS[name as ToolName];
    if (!toolHandler) throw new Error(`no handler registered for semantic tool: ${name}`);
    return toolHandler(input as ToolInput);
  });
}
