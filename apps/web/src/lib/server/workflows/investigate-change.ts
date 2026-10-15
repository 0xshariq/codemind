import { runAgent } from "../agent/runtime";
import type { AgentProvider } from "../agent/provider";

export function investigateChange(
  provider: AgentProvider,
  question: string,
  toolCalls: readonly { name: string; input: unknown }[] = [],
  onToolStart?: (name: string) => void,
  onReasoningStart?: () => void,
) {
  return runAgent(provider, question, toolCalls, onToolStart, onReasoningStart);
}