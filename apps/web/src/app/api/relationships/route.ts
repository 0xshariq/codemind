import { NextResponse } from "next/server";
import { querySymbolRelationships } from "@/lib/server/rust-client/client";

export async function POST(request: Request) {
  const body = (await request.json()) as Record<string, unknown>;
  const context = body.context as Record<string, unknown> | undefined;
  if (
    typeof body.symbolId !== "string" ||
    !context ||
    typeof context.repositoryId !== "string" ||
    typeof context.workspaceId !== "string" ||
    typeof context.sourceSnapshotId !== "string" ||
    typeof context.graphVersion !== "string"
  ) {
    return NextResponse.json({ error: "symbolId and complete context are required" }, { status: 400 });
  }
  try {
    const result = await querySymbolRelationships({
      symbolId: body.symbolId,
      limit: typeof body.limit === "number" ? body.limit : 20,
      context: {
        repositoryId: context.repositoryId,
        workspaceId: context.workspaceId,
        sourceSnapshotId: context.sourceSnapshotId,
        graphVersion: context.graphVersion,
      },
    });
    return NextResponse.json(result);
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Rust engine unavailable" },
      { status: 502 },
    );
  }
}
