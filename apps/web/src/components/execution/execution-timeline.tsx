import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { OperationStatus } from "./operation-status";

export type TimelineEvent = {
  sequence: number;
  status: string;
  message?: string;
  timestamp: string;
  engine?: string;
  stage?: string;
  stageState?: string;
  progressSequence?: number;
  progressEventId?: string;
};

export function ExecutionTimeline({ events }: { events: readonly TimelineEvent[] }) {
  const ordered = [...events].sort((left, right) => left.sequence - right.sequence);
  return (
    <Card>
      <CardHeader><CardTitle>Investigation execution</CardTitle></CardHeader>
      <CardContent role="region" aria-label="Scrollable investigation events" tabIndex={0} className="grid max-h-[32rem] gap-3 overflow-y-auto overscroll-contain pr-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
        {ordered.length === 0 && <p className="text-sm text-muted-foreground">No operation events have been returned.</p>}
        <ol aria-label="Ordered operation events" className="grid gap-3">
        {ordered.map((item) => (
          <li key={item.sequence} className="flex items-center justify-between gap-4 border-b border-border/60 pb-2 last:border-0">
            <div>
              <p className="text-sm">{item.message || item.status.replaceAll("_", " ")}</p>
              <p className="font-mono text-xs text-muted-foreground">#{item.sequence} · <time dateTime={item.timestamp}>{item.timestamp}</time></p>
            </div>
            <OperationStatus status={item.status} />
          </li>
        ))}
        </ol>
      </CardContent>
    </Card>
  );
}
