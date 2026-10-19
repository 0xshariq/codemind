import { LoaderCircle } from "lucide-react";

export function LoadingState({
  label = "Loading",
  detail,
  staged = false,
}: {
  label?: string;
  detail?: string;
  staged?: boolean;
}) {
  return (
    <div role="status" aria-live="polite" aria-busy="true" className="grid gap-3 rounded-xl border border-border bg-surface p-5">
      <div className="flex items-center gap-3"><LoaderCircle aria-hidden="true" className="size-5 animate-spin text-primary motion-reduce:animate-none" /><span className="text-sm font-medium">{label}</span></div>
      {detail && <p className="text-sm text-muted-foreground">{detail}</p>}
      {staged && <div aria-hidden="true" className="grid gap-2 sm:grid-cols-3">{[0, 1, 2].map((step) => <div key={step} className="h-1.5 rounded-full bg-muted" />)}</div>}
    </div>
  );
}
