export type GuardrailState = {
  calls: number;
  seen: Set<string>;
  perToolCalls: Map<string, number>;
  contexts: Set<string>;
  resultBytes: number;
};

export const MAX_TOOL_CALLS = 20;
export const MAX_TOOL_RESULT_BYTES = 256 * 1024;
export const MAX_CONTEXTS_PER_OPERATION = 1;

export function createGuardrailState(): GuardrailState {
  return { calls: 0, seen: new Set(), perToolCalls: new Map(), contexts: new Set(), resultBytes: 0 };
}

export function checkGuardrails(state: GuardrailState, key: string, toolName: string, maxCalls: number, contextKey?: string): void {
  if (state.calls >= MAX_TOOL_CALLS) throw new Error("tool-call budget exhausted");
  if (state.seen.has(key)) throw new Error("duplicate tool call rejected");
  if ((state.perToolCalls.get(toolName) ?? 0) >= maxCalls) throw new Error(`tool call limit exhausted for ${toolName}`);
  if (contextKey) {
    state.contexts.add(contextKey);
    if (state.contexts.size > MAX_CONTEXTS_PER_OPERATION) throw new Error("tool context budget exceeded");
  }
  state.seen.add(key);
  state.perToolCalls.set(toolName, (state.perToolCalls.get(toolName) ?? 0) + 1);
  state.calls += 1;
}

export function accountToolResult(state: GuardrailState, serialized: string): void {
  const bytes = Buffer.byteLength(serialized, "utf8");
  if (bytes > MAX_TOOL_RESULT_BYTES || state.resultBytes + bytes > MAX_TOOL_RESULT_BYTES) {
    throw new Error("tool result budget exceeded");
  }
  state.resultBytes += bytes;
}
