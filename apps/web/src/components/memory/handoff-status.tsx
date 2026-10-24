import { ArrowRight, CheckCircle2, CircleAlert, Clock3 } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

const statusMeta = { pending: { label: "Waiting to be claimed", Icon: Clock3, tone: "text-muted-foreground" }, claimed: { label: "Claimed by receiving stage", Icon: Clock3, tone: "text-primary" }, consumed: { label: "Consumed by receiving stage", Icon: CheckCircle2, tone: "text-success" }, expired: { label: "Expired", Icon: CircleAlert, tone: "text-danger" } } as const;

export function HandoffStatus({ handoffId, status, targetStage, sourceStage }: { handoffId?: string; status?: keyof typeof statusMeta; targetStage?: string; sourceStage?: string }) {
  const meta = status ? statusMeta[status] : undefined;
  const Icon = meta?.Icon ?? Clock3;
  return <Card aria-label="Handoff status"><CardHeader><CardTitle className="text-base">Persisted Handoff</CardTitle><CardDescription>Handoff lifecycle is independent from operation lifecycle.</CardDescription></CardHeader><CardContent className="grid gap-3 text-sm">{!handoffId ? <p className="text-muted-foreground">No handoff has been persisted for this investigation.</p> : <><div className="flex flex-wrap items-center gap-2"><code className="break-all text-xs">{handoffId}</code>{meta && <Badge variant={status === "consumed" ? "default" : status === "expired" ? "destructive" : "outline"}><Icon className="size-3" />{meta.label}</Badge>}</div>{sourceStage && targetStage && <div className="flex flex-wrap items-center gap-2 rounded-lg border border-border bg-surface p-3"><span>{sourceStage}</span><ArrowRight className="size-4 text-primary" /><span>{targetStage}</span></div>}{meta && <p className={`text-xs ${meta.tone}`}>{meta.label}. No UI state is inferred when the server has not persisted a handoff.</p>}</>}</CardContent></Card>;
}
