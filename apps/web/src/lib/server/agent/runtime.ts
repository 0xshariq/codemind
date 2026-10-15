import type { AgentMessage, AgentProvider } from "./provider";
import { SYSTEM_PROMPT } from "./prompts";
import { executeToolLoop } from "./tool-loop";

export async function runAgent(
  provider: AgentProvider,
  question: string,
  toolCalls: readonly { name: string; input: unknown }[] = [],
): Promise<string> {
  const evidence = await executeToolLoop(toolCalls);
  const messages: AgentMessage[] = [
    { role: "user", content: `${SYSTEM_PROMPT}\nQuestion: ${question}` },
    ...evidence.map((content) => ({ role: "tool" as const, content: String(content) })),
  ];
  return provider.complete(messages);
}