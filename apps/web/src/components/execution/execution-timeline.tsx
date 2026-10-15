import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { OperationStatus } from "./operation-status";

export type TimelineEvent = {
  sequence: number;
  status: string;
  message?: string;
  timestamp: string;
};

export function ExecutionTimeline({ events }: { events: readonly TimelineEvent[] }) {
  return (
    <Card>
      <CardHeader><CardTitle>Investigation execution</CardTitle></CardHeader>
      <CardContent className="grid gap-3">
        {events.length === 0 && <p className="text-sm text-muted-foreground">No operation started.</p>}
        {events.map((item) => (
          <div key={item.sequence} className="flex items-center justify-between gap-4 border-b border-border/60 pb-2 last:border-0">
            <div>
              <p className="text-sm">{item.message ?? item.status}</p>
              <p className="font-mono text-xs text-muted-foreground">{item.timestamp}</p>
            </div>
            <OperationStatus status={item.status} />
          </div>
        ))}
      </CardContent>
    </Card>
  );
}