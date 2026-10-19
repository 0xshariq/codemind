import { FlaskConical } from "lucide-react";
import { EmptyState } from "@/components/common/empty-state";
import { Badge } from "@/components/ui/badge";

export type TestImpactItem = { id: string; name: string; path: string; testType?: string; priority?: string; evidence?: string; snapshotId?: string };

export function TestImpactPanel({
  tests,
  availability = "available",
  currentSnapshotId,
}: {
  tests: readonly TestImpactItem[];
  availability?: "available" | "unsupported" | "unavailable";
  currentSnapshotId?: string;
}) {
  if (availability !== "available") return <EmptyState title={availability === "unsupported" ? "Test impact unsupported" : "Test impact unavailable"} description="TIE did not return test-impact candidates for this analysis." icon={<FlaskConical className="size-6" />} />;
  if (!tests.length) return <EmptyState title="No test candidates returned" description="No test impact candidates were supported by the available graph evidence." icon={<FlaskConical className="size-6" />} />;
  const unique = [...new Map(tests.map((test) => [test.id, test])).values()].slice(0, 200);
  return <section className="grid gap-3" aria-label="Test impact"><ul className="grid gap-2">{unique.map((test) => {
    const compatible = !currentSnapshotId || !test.snapshotId || currentSnapshotId === test.snapshotId;
    return <li key={test.id} className="grid gap-2 rounded-lg border border-border bg-surface p-3"><div className="flex flex-wrap items-center gap-2"><FlaskConical aria-hidden="true" className="size-4 text-primary" /><span className="text-sm font-medium">{test.name}</span>{test.testType && <Badge variant="outline">{test.testType.replaceAll("_", " ")}</Badge>}{test.priority && <Badge variant="secondary">{test.priority.replaceAll("_", " ")}</Badge>}</div><code className="break-all font-mono text-xs text-muted-foreground">{test.path}</code>{test.evidence && <p className="break-all text-xs text-muted-foreground">Evidence: <code>{test.evidence}</code></p>}<p className="text-xs text-muted-foreground">{compatible ? "Snapshot compatible" : "Candidate comes from a different snapshot"}{test.snapshotId ? ` · ${test.snapshotId}` : " · snapshot provenance unknown"}</p></li>;
  })}</ul></section>;
}
