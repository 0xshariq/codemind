import { NextResponse } from "next/server";
import { createConfiguredProvider } from "@/lib/server/agent/provider";
import { investigateChange } from "@/lib/server/workflows/investigate-change";
import { publishOperation } from "@/lib/server/operations/service";
import { setOperationResult } from "@/lib/server/operations/service";

export async function POST(request: Request) {
  let body: {
    question?: unknown;
    toolCalls?: unknown;
  };
  try {
    body = (await request.json()) as typeof body;
  } catch {
    return NextResponse.json({ error: "invalid JSON request body" }, { status: 400 });
  }
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
    let sequence = 1;
    try {
      const answer = await investigateChange(
        createConfiguredProvider(),
        body.question as string,
        toolCalls,
        (name) => {
          const stage = name === "get_test_intelligence" ? "tie" : name === "analyze_change" || name === "get_impact" ? "chie" : "coie";
          sequence += 1;
          publishOperation({ operationId, status: stage, sequence, message: stage === "coie" ? "Collecting canonical code evidence" : stage === "tie" ? "Collecting test intelligence" : "Computing change impact" });
        },
        () => {
          sequence += 1;
          publishOperation({ operationId, status: "reasoning", sequence, message: "Preparing evidence-grounded response" });
        },
      );
      sequence += 1;
      setOperationResult(operationId, { answer });
      publishOperation({ operationId, status: "completed", sequence, message: "Investigation completed" });
    } catch (error) {
      const message = error instanceof Error ? error.message : "Agent unavailable";
      sequence += 1;
      setOperationResult(operationId, { error: message });
      publishOperation({ operationId, status: "failed", sequence, message });
    }
  })();
  return NextResponse.json({ operationId }, { status: 202 });
}