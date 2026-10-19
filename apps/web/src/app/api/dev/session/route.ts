import { NextResponse } from "next/server";
import { createAuthenticatedSessionCookie } from "@/lib/server/auth/context";
import { getServerConfig } from "@/lib/server/config/env";

export async function POST(request: Request) {
  let config;
  try { config = getServerConfig(); }
  catch { return NextResponse.json({ error: "Development session configuration is invalid." }, { status: 503 }); }
  if (!config.developmentAuthEnabled || !config.developmentUserId || !config.developmentWorkspaceId) {
    return NextResponse.json({ error: "Development session bootstrap is disabled." }, { status: 404 });
  }
  const origin = request.headers.get("origin");
  const host = request.headers.get("host");
  let sameOrigin = false;
  try {
    const originUrl = origin ? new URL(origin) : null;
    const requestUrl = new URL(request.url);
    const forwardedProtocol = request.headers.get("x-forwarded-proto")?.split(",")[0]?.trim();
    const requestProtocol = forwardedProtocol ? `${forwardedProtocol}:` : requestUrl.protocol;
    sameOrigin = Boolean(originUrl && host
      && originUrl.host.toLowerCase() === host.toLowerCase()
      && originUrl.protocol === requestProtocol
      && !originUrl.username && !originUrl.password);
  } catch {
    sameOrigin = false;
  }
  if (!sameOrigin) {
    return NextResponse.json({ error: "Development session bootstrap requires a same-origin request." }, { status: 403 });
  }

  try {
    const cookie = createAuthenticatedSessionCookie({
      userId: config.developmentUserId,
      workspaceId: config.developmentWorkspaceId,
      allowedScopes: config.developmentScopes,
      teamIds: config.developmentTeamIds,
    });
    const response = NextResponse.json({
      outcome: "success",
      workspaceId: config.developmentWorkspaceId,
      expiresInSeconds: 8 * 60 * 60,
    }, { headers: { "cache-control": "no-store" } });
    response.headers.append("set-cookie", cookie);
    return response;
  } catch {
    return NextResponse.json({ error: "Development session could not be created." }, { status: 503 });
  }
}
