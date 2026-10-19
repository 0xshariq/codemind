"use client";

import { Badge } from "@/components/ui/badge";

function statusVariant(status: string) {
  const normalized = status.toLowerCase();
  if (["completed", "complete", "success"].includes(normalized)) return "default" as const;
  if (["failed", "failure", "unavailable"].includes(normalized)) return "destructive" as const;
  if (["partial", "truncated", "unknown"].includes(normalized)) return "secondary" as const;
  return "outline" as const;
}

export function OperationStatus({
  status,
  outcome,
  completeness,
}: {
  status: string;
  outcome?: string;
  completeness?: string;
}) {
  return <div className="flex flex-wrap items-center gap-1.5" aria-label={[status, outcome, completeness].filter(Boolean).join(", ")}>
    <Badge variant={statusVariant(status)}>{status.replaceAll("_", " ")}</Badge>
    {outcome && <Badge variant={statusVariant(outcome)}>{outcome.replaceAll("_", " ")}</Badge>}
    {completeness && <Badge variant={statusVariant(completeness)}>{completeness.replaceAll("_", " ")}</Badge>}
  </div>;
}
