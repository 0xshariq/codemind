import { ArrowDownRight, Network } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export function ImpactCard({
  directCount,
  indirectCount,
  paths,
  completeness,
  onSelectPath,
}: {
  directCount?: number;
  indirectCount?: number;
  paths: readonly { id: string; label: string; classification: string; depth: number }[];
  completeness: string;
  onSelectPath?: (id: string) => void;
}) {
  return <Card className="border-border bg-card">
    <CardHeader><CardTitle className="flex items-center gap-2"><Network aria-hidden="true" className="size-5 text-primary" />Change impact</CardTitle><CardDescription>Canonical direct and indirect paths are shown separately from risk.</CardDescription></CardHeader>
    <CardContent className="grid gap-3">
      <div className="flex flex-wrap gap-2">{directCount !== undefined && <Badge variant="outline">{directCount} direct</Badge>}{indirectCount !== undefined && <Badge variant="secondary">{indirectCount} indirect</Badge>}<Badge variant="outline">{completeness.replaceAll("_", " ")}</Badge></div>
      {paths.length === 0 ? <p className="text-sm text-muted-foreground">No impact paths were returned.</p> : <ul className="grid gap-2">{paths.slice(0, 200).map((path) => <li key={path.id}><button type="button" onClick={() => onSelectPath?.(path.id)} disabled={!onSelectPath} className="flex min-h-11 w-full items-center justify-between gap-3 rounded-md border border-border/70 px-3 py-2 text-left disabled:cursor-default focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"><span className="flex min-w-0 items-center gap-2"><ArrowDownRight aria-hidden="true" className="size-4 shrink-0 text-primary" /><code className="break-all text-sm">{path.label}</code></span><span className="shrink-0 text-xs text-muted-foreground">{path.classification.replaceAll("_", " ")} · depth {path.depth}</span></button></li>)}</ul>}
    </CardContent>
  </Card>;
}
