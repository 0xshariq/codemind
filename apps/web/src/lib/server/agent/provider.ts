import { generateText } from "ai";
import { createOpenRouter } from "@openrouter/ai-sdk-provider";

export type AgentMessage = { role: "user" | "assistant" | "tool"; content: string };

export type AgentProvider = {
  complete(messages: readonly AgentMessage[], systemPrompt?: string): Promise<string>;
};

const MAX_INPUT_CHARACTERS = 120_000;
const MAX_OUTPUT_TOKENS = 2_000;

function validateModel(model: string): string {
  const normalized = model.trim();
  if (normalized.length > 200 || !/^[a-zA-Z0-9_.-]+\/[a-zA-Z0-9_.:/-]+$/.test(normalized)) {
    throw new Error("CODEMIND_LLM_MODEL must be a valid provider/model identifier");
  }
  return normalized;
}

export function createConfiguredProvider(
  env: NodeJS.ProcessEnv = process.env,
  generate: typeof generateText = generateText,
): AgentProvider {
  const apiKey = env.OPENROUTER_API_KEY;
  const model = validateModel(env.CODEMIND_LLM_MODEL ?? "openrouter/free");
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
      const formattedMessages = messages.map((message) => ({
        role: message.role === "tool" ? "user" as const : message.role,
        content: message.content,
      }));
      const inputCharacters = formattedMessages.reduce((total, message) => total + message.content.length, systemPrompt?.length ?? 0);
      if (inputCharacters > MAX_INPUT_CHARACTERS) {
        throw new Error("Agent input exceeds the configured safety limit");
      }

      try {
        const result = await generate({
          model: router(model),
          system: systemPrompt,
          messages: formattedMessages,
          maxOutputTokens: MAX_OUTPUT_TOKENS,
        });
        return result.text;
      } catch (error) {
        const details = error && typeof error === "object" ? error as Record<string, unknown> : {};
        const statusCode = typeof details.statusCode === "number" && Number.isInteger(details.statusCode) ? details.statusCode
          : details.response && typeof details.response === "object" && typeof (details.response as Record<string, unknown>).status === "number"
            ? (details.response as Record<string, number>).status : undefined;
        throw new Error(`Configured language model request failed${statusCode ? ` (HTTP ${statusCode})` : ""}`);
      }
    },
  };
}
