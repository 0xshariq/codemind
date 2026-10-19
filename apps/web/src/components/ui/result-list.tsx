import type { ReactNode } from "react";
import { EmptyState } from "@/components/common/empty-state";

export type ResultListItem = { id: string; label: string; description?: string; trailing?: ReactNode };

export function ResultList({
  items,
  emptyTitle = "No results",
  emptyDescription = "No supported results were returned.",
  renderItem,
}: {
  items: readonly ResultListItem[];
  emptyTitle?: string;
  emptyDescription?: string;
  renderItem?: (item: ResultListItem) => ReactNode;
}) {
  if (items.length === 0) return <EmptyState title={emptyTitle} description={emptyDescription} />;
  return <ul className="grid gap-2" aria-label="Results">
    {items.slice(0, 500).map((item) => <li key={item.id} className="min-w-0 rounded-lg border border-border bg-surface px-3 py-2">
      {renderItem ? renderItem(item) : <div className="flex min-w-0 items-start justify-between gap-3"><div className="min-w-0"><p className="break-words text-sm font-medium">{item.label}</p>{item.description && <p className="mt-1 break-words text-xs text-muted-foreground">{item.description}</p>}</div>{item.trailing}</div>}
    </li>)}
  </ul>;
}
