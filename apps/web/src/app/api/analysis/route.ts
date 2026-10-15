import { NextResponse } from "next/server";
import { analyzeChange } from "@/lib/server/rust-client/client";
import { toolInputSchema } from "@/lib/server/tools/schemas";

export async function POST(request: Request) {
  const parsed = toolInputSchema.safeParse(await request.json());
  if (!parsed.success || !parsed.data.symbolId) {
    return NextResponse.json({ outcome: "invalid", error: "symbolId and complete context are required" }, { status: 400 });
  }
  try {
    const result = await analyzeChange({
      symbolId: parsed.data.symbolId,
      maxDepth: 4,
      context: parsed.data.context,
    });
    return NextResponse.json(result);
  } catch (error) {
    return NextResponse.json(
      { outcome: "unavailable", error: error instanceof Error ? error.message : "Rust engine unavailable" },
      { status: 502 },
    );
  }
}
