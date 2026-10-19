import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

const statusLabel: Record<"pending" | "claimed" | "consumed" | "expired", string> = {
  pending: "Waiting to be claimed",
  claimed: "Claimed by the receiving stage",
  consumed: "Consumed by the receiving stage",
  expired: "Expired",
};

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
      <CardHeader><CardTitle>Persisted handoff</CardTitle></CardHeader>
      <CardContent className="grid gap-3 text-sm">
        <div className="flex flex-wrap items-center gap-2">{handoffId ? <code className="break-all text-xs">{handoffId}</code> : <span className="text-muted-foreground">No handoff has been persisted.</span>}{status && <Badge variant={status === "consumed" ? "default" : status === "expired" ? "destructive" : "outline"}>{status}</Badge>}</div>
        {status && <p className="text-xs text-muted-foreground">{statusLabel[status]}. This handoff lifecycle is separate from the investigation operation lifecycle.</p>}
        {targetStage && <p><span className="text-muted-foreground">Receiving stage: </span><span>{targetStage}</span></p>}
      </CardContent>
    </Card>
  );
}
