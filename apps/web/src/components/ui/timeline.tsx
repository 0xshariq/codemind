import type { ReactNode } from "react";

export type TimelineItem = { id: string; label: string; detail?: string; timestamp?: string; icon?: ReactNode };

export function Timeline({ items, emptyMessage = "No timeline events are available." }: { items: readonly TimelineItem[]; emptyMessage?: string }) {
  if (!items.length) return <p className="rounded-lg border border-dashed border-border px-4 py-6 text-center text-sm text-muted-foreground">{emptyMessage}</p>;
  const visibleItems = items.slice(0, 500);
  return <ol className="grid" aria-label="Timeline">
    {visibleItems.map((item, index) => <li key={item.id} className="relative grid grid-cols-[1.5rem_minmax(0,1fr)] gap-x-3 pb-6 last:pb-0">
      {index < visibleItems.length - 1 && <span aria-hidden="true" className="absolute left-2.75 top-6 h-[calc(100%-1.5rem)] w-px bg-border" />}
      <span aria-hidden="true" className="z-10 grid size-6 place-items-center rounded-full border border-border bg-surface text-primary">{item.icon ?? <span className="size-1.5 rounded-full bg-current" />}</span>
      <div className="min-w-0"><p className="text-sm font-medium">{item.label}</p>{item.detail && <p className="mt-1 text-sm text-muted-foreground">{item.detail}</p>}{item.timestamp && <time dateTime={item.timestamp} className="mt-1 block font-mono text-xs text-muted-foreground">{item.timestamp}</time>}</div>
    </li>)}
  </ol>;
}
