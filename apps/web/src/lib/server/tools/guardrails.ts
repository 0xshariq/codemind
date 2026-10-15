export type GuardrailState = {
  calls: number;
  startedAt: number;
  seen: Set<string>;
};

export const MAX_TOOL_CALLS = 20;
export const OPERATION_DEADLINE_MS = 30_000;

export function createGuardrailState(): GuardrailState {
  return { calls: 0, startedAt: Date.now(), seen: new Set() };
}

export function checkGuardrails(state: GuardrailState, key: string): void {
  if (state.calls >= MAX_TOOL_CALLS) throw new Error("tool-call budget exhausted");
  if (Date.now() - state.startedAt > OPERATION_DEADLINE_MS) {
    throw new Error("operation deadline exceeded");
  }
  if (state.seen.has(key)) throw new Error("duplicate tool call rejected");
  state.seen.add(key);
  state.calls += 1;
}