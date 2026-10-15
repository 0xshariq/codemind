import { type ToolInput } from "./schemas";
import { checkGuardrails, createGuardrailState, type GuardrailState } from "./guardrails";
import { getTool } from "./registry";
import { TOOL_HANDLERS } from "./implementations";

export type ToolExecutor = (name: string, input: unknown, state?: GuardrailState) => Promise<unknown>;

export function createToolExecutor(handler: ToolExecutor): ToolExecutor {
  const state = createGuardrailState();
  return async (name, input, sharedState = state) => {
    const tool = getTool(name);
    const parsed: ToolInput = tool.input.parse(input);
    checkGuardrails(sharedState, `${tool.name}:${JSON.stringify(parsed)}`);
    return handler(tool.name, parsed, sharedState);
  };
}

export function createRegisteredToolExecutor(): ToolExecutor {
  return createToolExecutor(async (name, input) => {
    const toolHandler = TOOL_HANDLERS[name];
    if (!toolHandler) throw new Error(`no handler registered for semantic tool: ${name}`);
    return toolHandler(input as ToolInput);
  });
}