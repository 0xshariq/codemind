import { generateText } from "ai";
import { createOpenRouter } from "@openrouter/ai-sdk-provider";

export type AgentMessage = { role: "user" | "assistant" | "tool"; content: string };

export type AgentProvider = {
  complete(messages: readonly AgentMessage[], systemPrompt?: string): Promise<string>;
};

export function createConfiguredProvider(
  env: NodeJS.ProcessEnv = process.env,
): AgentProvider {
  const apiKey = env.OPENROUTER_API_KEY;
  const model = env.CODEMIND_LLM_MODEL ?? "openai/gpt-4o-mini";
  if (!apiKey) {
    return {
      async complete() {
        throw new Error("OPENROUTER_API_KEY is not configured");
      },
    };
  }
  const router = createOpenRouter({ apiKey });
  return {
    async complete(messages, systemPrompt) {
      const result = await generateText({
        model: router(model),
        system: systemPrompt,
        messages: messages.map((message) => ({
          role: message.role === "tool" ? "user" : message.role,
          content: message.content,
        })),
      });
      return result.text;
    },
  };
}