import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export function HandoffStatus({
  handoffId,
  status,
  targetStage,
}: {
  handoffId?: string;
  status?: "pending" | "claimed" | "consumed" | "expired";
  targetStage?: string;
}) {
  return (
    <Card aria-label="Handoff status">
      <CardHeader><CardTitle>Investigation handoff</CardTitle></CardHeader>
      <CardContent className="flex items-center justify-between gap-3 text-sm">
        {handoffId ? <code>{handoffId}</code> : <span className="text-muted-foreground">No handoff persisted</span>}
        {status && <Badge variant={status === "consumed" ? "default" : "outline"}>{status}</Badge>}
        {targetStage && <span className="text-muted-foreground">→ {targetStage}</span>}
      </CardContent>
    </Card>
  );
}