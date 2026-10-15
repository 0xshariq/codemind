"use client";

import { FormEvent, useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Textarea } from "@/components/ui/textarea";
import { ExecutionTimeline, type TimelineEvent } from "../execution/execution-timeline";

type Context = {
  repositoryId: string;
  workspaceId: string;
  sourceSnapshotId: string;
  graphVersion: string;
};

export function ChatPanel({ context, symbolId }: { context: Context; symbolId: string }) {
  const [question, setQuestion] = useState("What could this affect?");
  const [answer, setAnswer] = useState("");
  const [events, setEvents] = useState<TimelineEvent[]>([]);
  const [error, setError] = useState("");
  const [running, setRunning] = useState(false);

  async function submit(event: FormEvent) {
    event.preventDefault();
    setError("");
    setAnswer("");
    setRunning(true);
    const response = await fetch("/api/agent", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        question,
        toolCalls: [{
          name: "analyze_change",
          input: { context, symbolId, limit: 20 },
        }],
      }),
    });
    const payload = (await response.json()) as { operationId?: string; error?: string };
    if (!response.ok || !payload.operationId) {
      setError(payload.error ?? "Investigation failed");
      setRunning(false);
      return;
    }
    const operationId = payload.operationId;
    const source = new EventSource(`/api/operations/${operationId}/events`);
    source.onmessage = async () => {
      const history = await fetch(`/api/operations/${operationId}`).then((value) => value.json()) as {
        events?: TimelineEvent[];
        result?: { answer?: string; error?: string };
      };
      setEvents(history.events ?? []);
      if (history.result?.answer) setAnswer(history.result.answer);
      if (history.result?.error) setError(history.result.error);
      const status = history.events?.at(-1)?.status;
      if (status === "completed" || status === "failed") {
        source.close();
        setRunning(false);
      }
    };
    source.onerror = () => {
      source.close();
      setRunning(false);
      setError("Operation progress stream unavailable");
    };
  }

  return (
    <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_360px]">
      <Card>
        <CardHeader><CardTitle>Ask CodeMind</CardTitle></CardHeader>
        <CardContent>
          <form onSubmit={submit} className="grid gap-3">
            <Textarea value={question} onChange={(event) => setQuestion(event.target.value)} aria-label="Investigation question" />
            <Button type="submit" disabled={running}>{running ? "Investigating..." : "Investigate change"}</Button>
          </form>
          {error && <p role="alert" className="mt-4 rounded-xl border border-destructive/40 p-3 text-sm text-destructive">{error}</p>}
          {answer && <div className="mt-6 whitespace-pre-wrap rounded-xl border border-border p-4 text-sm">{answer}</div>}
        </CardContent>
      </Card>
      <ExecutionTimeline events={events} />
    </div>
  );
}