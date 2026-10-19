import "server-only";

import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { createHash, randomUUID } from "node:crypto";
import { mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { resolveRepository } from "@/lib/server/rust-client/client";

const execFileAsync = promisify(execFile);

type Registration = {
  sourcePath: string;
  workspaceId: string;
  name?: string;
  description?: string;
  source?: string;
  url?: string;
  repositoryIdentity?: string;
};

export type ResolvedRepository = {
  repositoryId: string;
  name: string;
  readiness: "ready" | "partial" | "failed";
  source: string;
  description?: string;
  url?: string;
  sourceSnapshotId?: string;
  graphVersion?: string;
  fileCount?: number;
  limitations: string[];
};

function registrations(): Registration[] {
  const raw = process.env.CODEMIND_REPOSITORIES_JSON;
  if (!raw) return [];
  const parsed: unknown = JSON.parse(raw);
  if (!Array.isArray(parsed) || parsed.length > 100) throw new Error("repository registration configuration is invalid");
  const configured = parsed.flatMap((item) => {
    if (!item || typeof item !== "object" || Array.isArray(item)) return [];
    const value = item as Record<string, unknown>;
    if (typeof value.sourcePath !== "string" || !value.sourcePath.trim() || typeof value.workspaceId !== "string" || !value.workspaceId.trim()) return [];
    return [{
      sourcePath: value.sourcePath,
      workspaceId: value.workspaceId,
      name: typeof value.name === "string" ? value.name : undefined,
      description: typeof value.description === "string" ? value.description : undefined,
      source: typeof value.source === "string" ? value.source : "local",
      url: typeof value.url === "string" ? value.url : undefined,
      repositoryIdentity: typeof value.repositoryIdentity === "string" ? value.repositoryIdentity : undefined,
    }];
  });
  return configured;
}

function storageRoot() {
  return path.resolve(process.env.CODEMIND_REPOSITORY_STORAGE_ROOT ?? path.join(process.cwd(), ".codemind", "repositories"));
}

function parseGithubSource(value: string) {
  let url: URL;
  try { url = new URL(value); } catch { throw new Error("Provide a valid GitHub repository URL."); }
  if (url.protocol !== "https:" || url.hostname !== "github.com" || url.username || url.password || url.port || url.search || url.hash) {
    throw new Error("Only public HTTPS GitHub repository URLs are supported.");
  }
  const parts = url.pathname.split("/").filter(Boolean);
  if (parts.length !== 2 || !parts.every((part) => /^[A-Za-z0-9_.-]{1,100}$/.test(part))) {
    throw new Error("Provide a GitHub URL in the form https://github.com/owner/repository.");
  }
  const owner = parts[0];
  const name = parts[1].replace(/\.git$/i, "");
  if (!name || name === "." || name === "..") throw new Error("The GitHub repository name is invalid.");
  return { owner, name, url: `https://github.com/${owner}/${name}.git` };
}

async function importedRegistrations(): Promise<Registration[]> {
  try {
    const value: unknown = JSON.parse(await readFile(path.join(storageRoot(), "registry.json"), "utf8"));
    if (!Array.isArray(value)) return [];
    return value.filter((entry): entry is Registration => !!entry && typeof entry === "object"
      && typeof (entry as Registration).sourcePath === "string" && typeof (entry as Registration).workspaceId === "string");
  } catch { return []; }
}

async function saveImportedRegistration(registration: Registration) {
  const root = storageRoot();
  await mkdir(root, { recursive: true, mode: 0o700 });
  const registryPath = path.join(root, "registry.json");
  const current = await importedRegistrations();
  const next = [...current.filter((item) => !(item.workspaceId === registration.workspaceId && item.url === registration.url)), registration];
  const temporary = path.join(root, `.registry-${randomUUID()}.tmp`);
  await writeFile(temporary, JSON.stringify(next), { mode: 0o600, flag: "wx" });
  await rename(temporary, registryPath);
}

async function workspaceRegistrations(workspaceId: string) {
  return [
    ...registrations().filter((item) => item.workspaceId === workspaceId),
    ...(await importedRegistrations()).filter((item) => item.workspaceId === workspaceId),
  ];
}

async function resolveRegistration(
  registration: Registration,
  workspaceId: string,
  includeSourcePaths = false,
): Promise<ResolvedRepository & { sourcePaths?: string[] }> {
  const response = await resolveRepository({
    sourcePath: registration.sourcePath,
    context: { repositoryId: registration.repositoryIdentity ?? "", workspaceId },
  });
  const result = response.snapshot as Record<string, unknown> | undefined;
  if (!result) throw new Error("repository resolver returned no snapshot");
  const rawContext = (result.context ?? {}) as Record<string, unknown>;
  const context = {
    repositoryId: String(rawContext.repositoryId ?? rawContext.repository_id ?? ""),
    workspaceId: String(rawContext.workspaceId ?? rawContext.workspace_id ?? ""),
    sourceSnapshotId: String(rawContext.sourceSnapshotId ?? rawContext.source_snapshot_id ?? ""),
    graphVersion: String(rawContext.graphVersion ?? rawContext.graph_version ?? ""),
  };
  if (!context.repositoryId || context.workspaceId !== workspaceId || !context.sourceSnapshotId || !context.graphVersion) {
    throw new Error("repository resolver returned an invalid context");
  }
  const manifestText = result.manifestJson ?? result.manifest_json;
  let fileCount: number | undefined;
  let sourcePaths: string[] | undefined;
  const limitations: string[] = [];
  if (typeof manifestText === "string") {
    try {
      const manifest = JSON.parse(manifestText) as Record<string, unknown>;
      const files = Array.isArray(manifest.files) ? manifest.files : [];
      const omitted = Array.isArray(manifest.omitted_files) ? manifest.omitted_files : Array.isArray(manifest.omittedFiles) ? manifest.omittedFiles : [];
      fileCount = files.length;
      if (includeSourcePaths) {
        sourcePaths = files.flatMap((entry) => {
          if (!entry || typeof entry !== "object" || Array.isArray(entry)) return [];
          const filePath = (entry as Record<string, unknown>).path;
          if (typeof filePath !== "string" || !filePath.trim() || filePath.length > 1_024
            || /[\\\u0000-\u001f\u007f]/.test(filePath) || path.posix.isAbsolute(filePath)
            || filePath.split("/").some((part) => part === "" || part === "." || part === "..")) return [];
          return [filePath];
        });
      }
      if (omitted.length > 0) limitations.push(`${omitted.length} source files were omitted by repository capture limits.`);
    } catch {
      limitations.push("The source manifest could not be summarized.");
    }
  }
  return {
    repositoryId: context.repositoryId,
    name: registration.name ?? String(result.repositoryName ?? result.repository_name ?? "Repository"),
    readiness: limitations.length ? "partial" : "ready",
    source: registration.source ?? "local",
    description: registration.description,
    url: registration.url,
    sourceSnapshotId: context.sourceSnapshotId,
    graphVersion: context.graphVersion,
    fileCount,
    limitations,
    ...(includeSourcePaths ? { sourcePaths: sourcePaths ?? [] } : {}),
  };
}

export async function listRepositories(workspaceId: string) {
  const configured = await workspaceRegistrations(workspaceId);
  const outcomes = await Promise.all(configured.map(async (item) => {
    try { return { state: "resolved" as const, repository: await resolveRegistration(item, workspaceId) }; }
    catch { return { state: "failed" as const, repository: null }; }
  }));
  const repositories = outcomes.flatMap((item) => item.repository ? [item.repository] : []);
  const hasFailures = outcomes.some((item) => item.state === "failed");
  return {
    outcome: repositories.length ? hasFailures ? "partial" : "success" : configured.length ? "failed" : "empty",
    repositories,
    limitations: configured.length === 0 ? ["No repositories are registered for this workspace."]
      : outcomes.some((item) => item.state === "failed") ? ["One or more registered repositories could not be resolved by the Rust source service."] : [],
  };
}

export async function getRepository(workspaceId: string, repositoryId: string) {
  const configured = await workspaceRegistrations(workspaceId);
  for (const item of configured) {
    try {
      const repository = await resolveRegistration(item, workspaceId);
      if (repository.repositoryId === repositoryId) return repository;
    } catch { /* Keep checking scoped registrations; details are not exposed to clients. */ }
  }
  return null;
}

export async function getRepositorySourcePath(workspaceId: string, repositoryId: string) {
  for (const registration of await workspaceRegistrations(workspaceId)) {
    try {
      const resolved = await resolveRegistration(registration, workspaceId);
      if (resolved.repositoryId === repositoryId) return registration.sourcePath;
    } catch { /* Do not disclose source paths or resolver errors to the caller. */ }
  }
  return null;
}

export async function getRepositoryInvestigationSource(workspaceId: string, repositoryId: string) {
  for (const registration of await workspaceRegistrations(workspaceId)) {
    try {
      const resolved = await resolveRegistration(registration, workspaceId, true);
      if (resolved.repositoryId !== repositoryId || !resolved.sourcePaths?.length) continue;
      const { sourcePaths, ...repository } = resolved;
      return { repository, sourcePath: registration.sourcePath, sourcePaths };
    } catch { /* Keep checking scoped registrations without exposing source paths or resolver errors. */ }
  }
  return null;
}

export async function prepareConfiguredRepository(input: {
  workspaceId: string;
  sourceType: "local" | "example" | "github";
  source: string;
  name?: string;
}) {
  if (input.sourceType === "github") {
    let repository: ReturnType<typeof parseGithubSource>;
    try { repository = parseGithubSource(input.source); }
    catch (error) { return { outcome: "failed", message: error instanceof Error ? error.message : "GitHub repository URL is invalid." }; }
    const configured = await workspaceRegistrations(input.workspaceId);
    const normalizedRepositoryUrl = repository.url.replace(/\.git$/i, "");
    const existing = configured.find((item) => item.url?.replace(/\.git$/i, "") === normalizedRepositoryUrl);
    if (existing) {
      try {
        const resolved = await resolveRegistration(existing, input.workspaceId);
        return { outcome: resolved.readiness === "partial" ? "partial" : "completed", repositoryId: resolved.repositoryId, message: "The GitHub source is already imported and resolved.", limitations: resolved.limitations };
      } catch { return { outcome: "failed", message: "The imported GitHub source could not be resolved." }; }
    }
    if (configured.filter((item) => item.source === "github").length >= 25) {
      return { outcome: "unsupported", message: "This workspace has reached the configured limit of 25 imported repositories." };
    }
    const root = storageRoot();
    const workspaceKey = createHash("sha256").update(input.workspaceId).digest("hex").slice(0, 32);
    const repositoryPath = path.join(root, workspaceKey, randomUUID());
    const token = process.env.CODEMIND_GITHUB_TOKEN?.trim();
    if (token && (token.length > 500 || /[\r\n]/.test(token))) {
      return { outcome: "failed", message: "GitHub authentication configuration is invalid." };
    }
    try {
      await mkdir(path.dirname(repositoryPath), { recursive: true, mode: 0o700 });
      const gitEnv: NodeJS.ProcessEnv = {
        NODE_ENV: process.env.NODE_ENV,
        PATH: process.env.PATH,
        GIT_TERMINAL_PROMPT: "0",
        GIT_CONFIG_NOSYSTEM: "1",
        GIT_CONFIG_GLOBAL: "/dev/null",
        GIT_CONFIG_COUNT: "0",
      };
      if (token) {
        gitEnv.GIT_CONFIG_COUNT = "1";
        gitEnv.GIT_CONFIG_KEY_0 = "http.https://github.com/.extraheader";
        const authorization = Buffer.from(`x-access-token:${token}`, "utf8").toString("base64");
        gitEnv.GIT_CONFIG_VALUE_0 = `Authorization: Basic ${authorization}`;
      }
      await execFileAsync("git", ["clone", "--depth", "1", "--single-branch", "--filter=blob:limit=2m", "--", repository.url, repositoryPath], {
        timeout: 300_000,
        maxBuffer: 1_000_000,
        windowsHide: true,
        env: gitEnv,
      });
    } catch {
      await rm(repositoryPath, { recursive: true, force: true }).catch(() => undefined);
      return {
        outcome: "failed",
        message: token
          ? "GitHub could not clone this repository. Check the URL, token access, and repository permissions."
          : "GitHub could not clone this repository. Check the URL and configure CODEMIND_GITHUB_TOKEN for private repositories.",
      };
    }
    const registration: Registration = {
      sourcePath: repositoryPath,
      workspaceId: input.workspaceId,
      name: input.name?.trim() || repository.name,
      source: "github",
      url: `https://github.com/${repository.owner}/${repository.name}`,
      repositoryIdentity: `github:${repository.owner.toLowerCase()}/${repository.name.toLowerCase()}`,
    };
    try {
      const resolved = await resolveRegistration(registration, input.workspaceId);
      await saveImportedRegistration(registration);
      return {
        outcome: resolved.readiness === "partial" ? "partial" : "completed",
        repositoryId: resolved.repositoryId,
        message: resolved.readiness === "partial" ? "GitHub source imported; repository capture reports omissions." : "GitHub source imported and its canonical source snapshot was resolved.",
        limitations: resolved.limitations,
      };
    } catch {
      await rm(repositoryPath, { recursive: true, force: true }).catch(() => undefined);
      return { outcome: "failed", message: "The repository was cloned, but the Rust source service could not create its snapshot. Check that the engine is running and try again." };
    }
  }
  const configured = await workspaceRegistrations(input.workspaceId);
  const candidates = configured.filter((item) => input.sourceType === "example"
    ? item.source === "example" && (!input.name || item.name?.toLowerCase() === input.name.toLowerCase())
    : item.sourcePath === input.source);
  if (candidates.length === 0) {
    return { outcome: "unsupported", message: "Only repository sources registered for this workspace can be prepared." };
  }
  if (candidates.length > 1) {
    return { outcome: "unsupported", message: "The configured source is ambiguous; provide its registered name." };
  }
  try {
    const repository = await resolveRegistration(candidates[0], input.workspaceId);
    return {
      outcome: repository.readiness === "partial" ? "partial" : "completed",
      repositoryId: repository.repositoryId,
      message: repository.readiness === "partial" ? "Source snapshot resolved with capture omissions." : "Configured source snapshot resolved by the Rust repository service.",
      limitations: repository.limitations,
    };
  } catch {
    return { outcome: "failed", message: "The configured source could not be resolved by the Rust repository service." };
  }
}
