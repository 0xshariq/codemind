import "server-only";

import { createHmac, randomUUID, timingSafeEqual } from "node:crypto";

const COOKIE_NAME = "codemind_session";
const SESSION_VERSION = 1;
const SESSION_TTL_SECONDS = 8 * 60 * 60;
const CLOCK_SKEW_SECONDS = 60;
const ALLOWED_SCOPES = new Set(["user", "repository", "team"]);

export type AuthenticatedSession = {
  version: typeof SESSION_VERSION;
  sessionId: string;
  userId: string;
  workspaceId: string;
  allowedScopes: string[];
  teamIds: string[];
  issuedAt: number;
  expiresAt: number;
};

type SessionInput = {
  userId: string;
  workspaceId: string;
  allowedScopes: readonly string[];
  teamIds?: readonly string[];
};

function secretKey(env: NodeJS.ProcessEnv): string {
  const secret = env.CODEMIND_SESSION_SECRET;
  if (!secret || Buffer.byteLength(secret, "utf8") < 32) {
    throw new Error("CODEMIND_SESSION_SECRET must contain at least 32 bytes");
  }
  return secret;
}

function validIdentity(value: unknown): value is string {
  return typeof value === "string"
    && value.trim().length > 0
    && Buffer.byteLength(value, "utf8") <= 512
    && !/[\u0000-\u001f\u007f]/.test(value);
}

function sign(payload: string, secret: string): string {
  return createHmac("sha256", secret).update(payload).digest("base64url");
}

function readCookie(request: Request): string {
  const values = (request.headers.get("cookie") ?? "").split(";")
    .map((part) => part.trim())
    .filter((part) => part.startsWith(`${COOKIE_NAME}=`));
  if (values.length !== 1) throw new Error("authenticated session is required");
  const token = values[0].slice(COOKIE_NAME.length + 1);
  if (token.length > 8_192) throw new Error("authenticated session is invalid");
  return token;
}

export function loadAuthenticatedSession(
  request: Request,
  env: NodeJS.ProcessEnv = process.env,
  nowSeconds = Math.floor(Date.now() / 1_000),
): AuthenticatedSession {
  const token = readCookie(request);
  const parts = token.split(".");
  if (parts.length !== 2 || !/^[A-Za-z0-9_-]+$/.test(parts[0]) || !/^[A-Za-z0-9_-]+$/.test(parts[1])) {
    throw new Error("authenticated session is invalid");
  }
  const [payload, signature] = parts;
  const expected = sign(payload, secretKey(env));
  if (signature.length !== expected.length
    || !timingSafeEqual(Buffer.from(signature, "ascii"), Buffer.from(expected, "ascii"))) {
    throw new Error("authenticated session is invalid");
  }

  let claims: unknown;
  try {
    const decoded = Buffer.from(payload, "base64url").toString("utf8");
    claims = JSON.parse(decoded);
  } catch {
    throw new Error("authenticated session is invalid");
  }
  if (!claims || typeof claims !== "object" || Array.isArray(claims)) {
    throw new Error("authenticated session is invalid");
  }
  const value = claims as Record<string, unknown>;
  const allowedScopes = value.allowedScopes;
  const teamIds = value.teamIds ?? [];
  if (value.version !== SESSION_VERSION
    || !validIdentity(value.sessionId)
    || !validIdentity(value.userId)
    || !validIdentity(value.workspaceId)
    || !Array.isArray(allowedScopes)
    || allowedScopes.length > ALLOWED_SCOPES.size
    || allowedScopes.some((scope) => typeof scope !== "string" || !ALLOWED_SCOPES.has(scope))
    || new Set(allowedScopes).size !== allowedScopes.length
    || !Array.isArray(teamIds)
    || teamIds.length > 100
    || teamIds.some((teamId) => !validIdentity(teamId))
    || typeof value.issuedAt !== "number"
    || typeof value.expiresAt !== "number"
    || !Number.isSafeInteger(value.issuedAt)
    || !Number.isSafeInteger(value.expiresAt)
    || value.issuedAt > nowSeconds + CLOCK_SKEW_SECONDS
    || value.expiresAt <= nowSeconds
    || value.expiresAt <= value.issuedAt
    || value.expiresAt - value.issuedAt > SESSION_TTL_SECONDS) {
    throw new Error("authenticated session is invalid or expired");
  }
  return {
    version: SESSION_VERSION,
    sessionId: value.sessionId as string,
    userId: value.userId as string,
    workspaceId: value.workspaceId as string,
    allowedScopes: [...allowedScopes] as string[],
    teamIds: [...teamIds] as string[],
    issuedAt: value.issuedAt,
    expiresAt: value.expiresAt,
  };
}

export function createAuthenticatedSessionCookie(
  input: SessionInput,
  options: { env?: NodeJS.ProcessEnv; nowSeconds?: number; secure?: boolean } = {},
): string {
  const env = options.env ?? process.env;
  const now = options.nowSeconds ?? Math.floor(Date.now() / 1_000);
  if (!validIdentity(input.userId) || !validIdentity(input.workspaceId)) {
    throw new Error("session user and workspace identities are required");
  }
  const allowedScopes = [...new Set(input.allowedScopes)];
  const teamIds = [...new Set(input.teamIds ?? [])];
  if (allowedScopes.length > ALLOWED_SCOPES.size
    || allowedScopes.some((scope) => !ALLOWED_SCOPES.has(scope))
    || teamIds.length > 100
    || teamIds.some((teamId) => !validIdentity(teamId))) {
    throw new Error("session authorization claims are invalid");
  }
  const claims: AuthenticatedSession = {
    version: SESSION_VERSION,
    sessionId: randomUUID(),
    userId: input.userId,
    workspaceId: input.workspaceId,
    allowedScopes,
    teamIds,
    issuedAt: now,
    expiresAt: now + SESSION_TTL_SECONDS,
  };
  const payload = Buffer.from(JSON.stringify(claims), "utf8").toString("base64url");
  const token = `${payload}.${sign(payload, secretKey(env))}`;
  const secure = options.secure ?? env.NODE_ENV === "production";
  return `${COOKIE_NAME}=${token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=${SESSION_TTL_SECONDS}${secure ? "; Secure" : ""}`;
}
