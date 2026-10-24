"use client";

import type { LucideIcon } from "lucide-react";

export type SettingsNavigationItem = { id: string; title: string; description: string; icon: LucideIcon };

export function SettingsNavigation({
  items,
  activeId,
  onSelect,
}: {
  items: SettingsNavigationItem[];
  activeId?: string;
  onSelect?: (id: string) => void;
}) {
  return (
    <nav aria-label="Settings sections" className="grid h-fit gap-1 rounded-xl border border-border bg-surface p-2 lg:sticky lg:top-6">
      {items.map(({ id, title, description, icon: Icon }) => {
        const active = activeId === id;
        return (
          <button
            key={id}
            type="button"
            onClick={() => onSelect?.(id)}
            className={`flex min-h-14 items-start gap-3 rounded-lg px-3 py-2 text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${active ? "border-l-2 border-primary bg-primary/10" : "hover:bg-surface-alt"}`}
            aria-current={active ? "location" : undefined}
          >
            <Icon aria-hidden="true" className={`mt-0.5 size-4 shrink-0 ${active ? "text-primary" : "text-accent"}`} />
            <span className="min-w-0">
              <span className="block text-sm font-medium">{title}</span>
              <span className="block truncate text-xs text-muted-foreground">{description}</span>
            </span>
          </button>
        );
      })}
    </nav>
  );
}
