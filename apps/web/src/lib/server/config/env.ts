export type ServerConfig = {
  rustEndpoint: string;
  rustTimeoutMs: number;
};

export function getServerConfig(
  env: NodeJS.ProcessEnv = process.env,
): ServerConfig {
  const rustTimeoutMs = Number(env.CODEMIND_ENGINE_TIMEOUT_MS ?? "5000");
  if (!Number.isInteger(rustTimeoutMs) || rustTimeoutMs <= 0) {
    throw new Error("CODEMIND_ENGINE_TIMEOUT_MS must be a positive integer");
  }
  return {
    rustEndpoint: env.CODEMIND_ENGINE_ENDPOINT ?? "127.0.0.1:50051",
    rustTimeoutMs,
  };
}