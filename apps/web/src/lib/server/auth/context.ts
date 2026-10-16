import "server-only";
import { createHmac, timingSafeEqual } from "node:crypto";

export type AuthenticatedSession = {
  userId: string;
  workspaceId: string;
  allowedScopes: string[];
};

function encodedSession(value: string) {
  return Buffer.from(value, "base64url").toString("utf8");
}

export function loadAuthenticatedSession(request: Request): AuthenticatedSession {
  const cookie = request.headers.get("cookie")
    ?.split(";")
    .map((value) => value.trim())
    .find((value) => value.startsWith("codemind_session="))
    ?.slice("codemind_session=".length);
  const secret = process.env.CODEMIND_SESSION_SECRET;
  if (!cookie || !secret) throw new Error("authenticated session is required");
  const [payload, signature] = cookie.split(".");
  if (!payload || !signature) throw new Error("authenticated session is invalid");
  const expected = createHmac("sha256", secret).update(payload).digest("base64url");
  if (signature.length !== expected.length
    || !timingSafeEqual(Buffer.from(signature), Buffer.from(expected))) {
    throw new Error("authenticated session is invalid");
  }
  const session = JSON.parse(encodedSession(payload)) as Partial<AuthenticatedSession>;
  if (
    typeof session.userId !== "string"
    || typeof session.workspaceId !== "string"
    || !Array.isArray(session.allowedScopes)
    || session.allowedScopes.some((scope) => typeof scope !== "string")
  ) {
    throw new Error("authenticated session is invalid");
  }
  return {
    userId: session.userId,
    workspaceId: session.workspaceId,
    allowedScopes: session.allowedScopes,
  };
}