import { NextResponse } from "next/server";

export async function GET(
  _request: Request,
  context: { params: Promise<{ repositoryId: string }> },
) {
  const { repositoryId } = await context.params;
  if (!repositoryId.trim()) {
    return NextResponse.json({ error: "repositoryId is required" }, { status: 400 });
  }
  const configured = process.env.CODEMIND_REPOSITORIES_JSON;
  if (configured) {
    try {
      const repositories = JSON.parse(configured) as unknown;
      if (Array.isArray(repositories)) {
        const repository = repositories.find(
          (item): item is { repositoryId?: string } =>
            !!item && typeof item === "object" && (item as { repositoryId?: unknown }).repositoryId === repositoryId,
        );
        if (repository) return NextResponse.json({ outcome: "success", repository });
        return NextResponse.json({ outcome: "empty", repositoryId }, { status: 404 });
      }
    } catch {
      return NextResponse.json({ outcome: "failed", error: "CODEMIND_REPOSITORIES_JSON is invalid" }, { status: 500 });
    }
  }
  return NextResponse.json({
    repositoryId,
    outcome: "unavailable",
    limitations: ["Configure CODEMIND_REPOSITORIES_JSON for local repository discovery."],
  });
}