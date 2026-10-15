import { NextResponse } from "next/server";
import { createConfiguredProvider } from "@/lib/server/agent/provider";
import { investigateChange } from "@/lib/server/workflows/investigate-change";
import { publishOperation } from "@/lib/server/operations/service";
import { setOperationResult } from "@/lib/server/operations/service";

export async function POST(request: Request) {
  const body = (await request.json()) as {
    question?: unknown;
    toolCalls?: unknown;
  };
  if (typeof body.question !== "string" || body.question.trim().length === 0) {
    return NextResponse.json({ error: "question is required" }, { status: 400 });
  }
  const operationId = crypto.randomUUID();
  const toolCalls = Array.isArray(body.toolCalls)
    ? body.toolCalls.filter(
        (call): call is { name: string; input: unknown } =>
          !!call &&
          typeof call === "object" &&
          typeof (call as { name?: unknown }).name === "string" &&
          "input" in call,
      )
    : [];
  publishOperation({ operationId, status: "planning", sequence: 1, message: "Planning investigation" });
  void (async () => {
    try {
      publishOperation({ operationId, status: "coie", sequence: 2, message: "Collecting canonical code evidence" });
      publishOperation({ operationId, status: "tie", sequence: 3, message: "Collecting test intelligence" });
      publishOperation({ operationId, status: "chie", sequence: 4, message: "Computing change impact" });
      const answer = await investigateChange(createConfiguredProvider(), body.question as string, toolCalls);
      publishOperation({ operationId, status: "reasoning", sequence: 5, message: "Preparing evidence-grounded response" });
      setOperationResult(operationId, { answer });
      publishOperation({ operationId, status: "completed", sequence: 6, message: "Investigation completed" });
    } catch (error) {
      const message = error instanceof Error ? error.message : "Agent unavailable";
      setOperationResult(operationId, { error: message });
      publishOperation({ operationId, status: "failed", sequence: 6, message });
    }
  })();
  return NextResponse.json({ operationId }, { status: 202 });
}