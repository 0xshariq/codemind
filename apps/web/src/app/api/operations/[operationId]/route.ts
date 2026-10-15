import { NextResponse } from "next/server";
import { getOperation, getOperationResult } from "@/lib/server/operations/service";

export async function GET(_request: Request, { params }: { params: Promise<{ operationId: string }> }) {
  const { operationId } = await params;
  return NextResponse.json({ operationId, events: getOperation(operationId), result: getOperationResult(operationId) });
}