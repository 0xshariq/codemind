import { NextResponse } from "next/server";

export function GET() {
  const configured = process.env.CODEMIND_REPOSITORIES_JSON;
  if (configured) {
    try {
      const repositories = JSON.parse(configured) as unknown;
      if (Array.isArray(repositories)) {
        return NextResponse.json({ repositories, outcome: repositories.length ? "success" : "empty" });
      }
    } catch {
      return NextResponse.json({ outcome: "failed", error: "CODEMIND_REPOSITORIES_JSON is invalid" }, { status: 500 });
    }
  }
  return NextResponse.json({
    repositories: [],
    outcome: "unavailable",
    limitations: ["Configure CODEMIND_REPOSITORIES_JSON for local repository discovery."],
  });
}