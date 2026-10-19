"use client";

import type { LucideIcon } from "lucide-react";
import { TabsList, TabsTrigger } from "@/components/ui/tabs";

export type SettingsNavigationItem = {
  id: string;
  title: string;
  description: string;
  icon: LucideIcon;
};

export function SettingsNavigation({ items }: { items: SettingsNavigationItem[] }) {
  return (
    <TabsList aria-label="Settings sections" className="grid h-fit w-full grid-cols-2 gap-1 rounded-xl border border-border bg-surface p-2 lg:grid-cols-1">
      {items.map(({ id, title, description, icon: Icon }) => (
        <TabsTrigger key={id} value={id} className="h-auto min-h-12 justify-start gap-3 rounded-lg px-3 py-2 text-left data-active:bg-primary/10 data-active:text-foreground">
          <Icon aria-hidden="true" className="size-4 shrink-0 text-accent" />
          <span className="min-w-0">
            <span className="block text-sm font-medium">{title}</span>
            <span className="block truncate text-xs text-muted-foreground">{description}</span>
          </span>
        </TabsTrigger>
      ))}
    </TabsList>
  );
}
