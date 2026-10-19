import type { ReactNode } from "react";
import { AlertCircle, RefreshCw } from "lucide-react";

export function ErrorState({
  title = "Something went wrong",
  description,
  onRetry,
  action,
}: {
  title?: string;
  description: string;
  onRetry?: () => void;
  action?: ReactNode;
}) {
  return (
    <section role="alert" className="grid justify-items-center gap-3 rounded-xl border border-danger/40 bg-danger/5 px-6 py-8 text-center">
      <AlertCircle aria-hidden="true" className="size-6 text-danger" />
      <h2 className="text-base font-semibold text-foreground">{title}</h2>
      <p className="max-w-lg text-sm leading-6 text-muted-foreground">{description}</p>
      {action ?? (onRetry && <button type="button" onClick={onRetry} className="inline-flex min-h-10 items-center gap-2 rounded-md border border-border bg-surface px-3 text-sm font-medium hover:bg-surface-alt focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"><RefreshCw aria-hidden="true" className="size-4" />Try again</button>)}
    </section>
  );
}
