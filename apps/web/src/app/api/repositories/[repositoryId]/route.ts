import { NextResponse } from "next/server";

export async function GET(
  _request: Request,
  context: { params: Promise<{ repositoryId: string }> },
) {
  const { repositoryId } = await context.params;
  if (!repositoryId.trim()) {
    return NextResponse.json({ error: "repositoryId is required" }, { status: 400 });
  }
  return NextResponse.json({
    repositoryId,
    outcome: "unsupported",
    limitations: ["Repository management is outside the Day 1 local-source slice."],
  }, { status: 501 });
}