export type GuardrailState = {
  calls: number;
  startedAt: number;
  seen: Set<string>;
  perToolCalls: Map<string, number>;
};

export const MAX_TOOL_CALLS = 20;
export const OPERATION_DEADLINE_MS = 30_000;

export function createGuardrailState(): GuardrailState {
  return { calls: 0, startedAt: Date.now(), seen: new Set(), perToolCalls: new Map() };
}

export function checkGuardrails(state: GuardrailState, key: string, toolName: string, maxCalls: number): void {
  if (state.calls >= MAX_TOOL_CALLS) throw new Error("tool-call budget exhausted");
  if (Date.now() - state.startedAt > OPERATION_DEADLINE_MS) {
    throw new Error("operation deadline exceeded");
  }
  if (state.seen.has(key)) throw new Error("duplicate tool call rejected");
  if ((state.perToolCalls.get(toolName) ?? 0) >= maxCalls) throw new Error(`tool call limit exhausted for ${toolName}`);
  state.seen.add(key);
  state.perToolCalls.set(toolName, (state.perToolCalls.get(toolName) ?? 0) + 1);
  state.calls += 1;
}