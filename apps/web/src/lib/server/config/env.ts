import "server-only";

import path from "node:path";

const DEFAULT_MODEL = "openrouter/free";
const ALLOWED_SCOPES = new Set(["user", "repository", "team"]);

export type TrustedRepositoryConfig = {
  sourcePath: string;
  workspaceId: string;
  name?: string;
  description?: string;
  source?: string;
  url?: string;
  repositoryIdentity?: string;
};

export type ServerConfig = {
  rustEndpoint: string;
  llmProvider: "openrouter";
  llmModel: string;
  openRouterApiKey?: string;
  sessionSecret?: string;
  githubToken?: string;
  repositoryStorageRoot: string;
  repositories: TrustedRepositoryConfig[];
  developmentAuthEnabled: boolean;
  developmentUserId?: string;
  developmentWorkspaceId?: string;
  developmentScopes: string[];
  developmentTeamIds: string[];
};

function optionalCredential(value: string | undefined, name: string, maxBytes: number): string | undefined {
  const normalized = value?.trim();
  if (!normalized) return undefined;
  if (Buffer.byteLength(normalized, "utf8") > maxBytes || /[\r\n\u0000]/.test(normalized)) {
    throw new Error(name + " is invalid");
  }
  return normalized;
}

function parseRepositories(raw: string | undefined): TrustedRepositoryConfig[] {
  if (!raw?.trim()) return [];
  let value: unknown;
  try { value = JSON.parse(raw); } catch { throw new Error("CODEMIND_REPOSITORIES_JSON must contain valid JSON"); }
  if (!Array.isArray(value) || value.length > 100) throw new Error("CODEMIND_REPOSITORIES_JSON must be an array of at most 100 repositories");
  return value.map((entry, index) => {
    if (!entry || typeof entry !== "object" || Array.isArray(entry)) throw new Error("repository registration " + index + " is invalid");
    const item = entry as Record<string, unknown>;
    if (typeof item.sourcePath !== "string" || !item.sourcePath.trim()
      || typeof item.workspaceId !== "string" || !item.workspaceId.trim()
      || /[\u0000-\u001f\u007f]/.test(item.workspaceId)) {
      throw new Error("repository registration " + index + " requires a sourcePath and workspaceId");
    }
    return {
      sourcePath: path.resolve(item.sourcePath),
      workspaceId: item.workspaceId,
      name: typeof item.name === "string" ? item.name.slice(0, 120) : undefined,
      description: typeof item.description === "string" ? item.description.slice(0, 500) : undefined,
      source: typeof item.source === "string" ? item.source.slice(0, 40) : undefined,
      url: typeof item.url === "string" ? item.url.slice(0, 2_048) : undefined,
      repositoryIdentity: typeof item.repositoryIdentity === "string" ? item.repositoryIdentity.slice(0, 512) : undefined,
    };
  });
}

function enabled(value: string | undefined, name: string): boolean {
  if (value === undefined || value === "") return false;
  if (value === "true") return true;
  if (value === "false") return false;
  throw new Error(name + " must be true or false");
}

export function getServerConfig(env: NodeJS.ProcessEnv = process.env): ServerConfig {
  const rustEndpoint = (env.CODEMIND_ENGINE_ENDPOINT ?? "127.0.0.1:50051").trim();
  if (!rustEndpoint || rustEndpoint.length > 256 || /[\s\u0000-\u001f]/.test(rustEndpoint)) {
    throw new Error("CODEMIND_ENGINE_ENDPOINT is invalid");
  }
  const llmModel = (env.CODEMIND_LLM_MODEL ?? DEFAULT_MODEL).trim();
  if (llmModel.length > 200 || !/^[a-zA-Z0-9_.-]+\/[a-zA-Z0-9_.:/-]+$/.test(llmModel)) {
    throw new Error("CODEMIND_LLM_MODEL must be a valid provider/model identifier");
  }
  const sessionSecret = optionalCredential(env.CODEMIND_SESSION_SECRET, "CODEMIND_SESSION_SECRET", 4_096);
  if (sessionSecret && Buffer.byteLength(sessionSecret, "utf8") < 32) {
    throw new Error("CODEMIND_SESSION_SECRET must contain at least 32 bytes");
  }
  const developmentAuthEnabled = enabled(env.CODEMIND_DEV_AUTH_ENABLED, "CODEMIND_DEV_AUTH_ENABLED");
  if (developmentAuthEnabled && env.NODE_ENV === "production") {
    throw new Error("CODEMIND_DEV_AUTH_ENABLED cannot be enabled in production");
  }
  const developmentScopes = (env.CODEMIND_DEV_SCOPES ?? "user,repository").split(",").map((scope) => scope.trim()).filter(Boolean);
  if (developmentScopes.some((scope) => !ALLOWED_SCOPES.has(scope))) throw new Error("CODEMIND_DEV_SCOPES contains an unsupported scope");
  const developmentTeamIds = (env.CODEMIND_DEV_TEAM_IDS ?? "").split(",").map((id) => id.trim()).filter(Boolean);
  if (developmentTeamIds.length > 100 || developmentTeamIds.some((id) => /[\u0000-\u001f\u007f]/.test(id) || id.length > 512)) {
    throw new Error("CODEMIND_DEV_TEAM_IDS is invalid");
  }
  const developmentUserId = optionalCredential(env.CODEMIND_DEV_USER_ID, "CODEMIND_DEV_USER_ID", 512);
  const developmentWorkspaceId = optionalCredential(env.CODEMIND_DEV_WORKSPACE_ID, "CODEMIND_DEV_WORKSPACE_ID", 512);
  if (developmentAuthEnabled && (!developmentUserId || !developmentWorkspaceId)) {
    throw new Error("development auth requires CODEMIND_DEV_USER_ID and CODEMIND_DEV_WORKSPACE_ID");
  }
  return {
    rustEndpoint,
    llmProvider: "openrouter",
    llmModel,
    openRouterApiKey: optionalCredential(env.OPENROUTER_API_KEY, "OPENROUTER_API_KEY", 1_024),
    sessionSecret,
    githubToken: optionalCredential(env.CODEMIND_GITHUB_TOKEN, "CODEMIND_GITHUB_TOKEN", 4_096),
    repositoryStorageRoot: path.resolve(env.CODEMIND_REPOSITORY_STORAGE_ROOT ?? path.join(process.cwd(), ".codemind", "repositories")),
    repositories: parseRepositories(env.CODEMIND_REPOSITORIES_JSON),
    developmentAuthEnabled,
    developmentUserId,
    developmentWorkspaceId,
    developmentScopes: [...new Set(developmentScopes)],
    developmentTeamIds,
  };
}
