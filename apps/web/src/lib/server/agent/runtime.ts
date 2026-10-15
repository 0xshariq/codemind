import type { AgentMessage, AgentProvider } from "./provider";
import { SYSTEM_PROMPT } from "./prompts";
import { executeToolLoop } from "./tool-loop";

export async function runAgent(
  provider: AgentProvider,
  question: string,
  toolCalls: readonly { name: string; input: unknown }[] = [],
  onToolStart?: (name: string) => void,
  onReasoningStart?: () => void,
): Promise<string> {
  const evidence = await executeToolLoop(toolCalls, 8, onToolStart);
  const messages: AgentMessage[] = [
    { role: "user", content: `Question: ${question}` },
    ...evidence.map((content) => ({ role: "tool" as const, content: String(content) })),
  ];
  onReasoningStart?.();
  return provider.complete(messages, SYSTEM_PROMPT);
}