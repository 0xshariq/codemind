"use client";

import { useEffect, useState, type FormEvent } from "react";
import { LoaderCircle, Search, SlidersHorizontal, X } from "lucide-react";
import { Modal, ModalContent, ModalDescription, ModalHeader, ModalTitle } from "@/components/ui/modal";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

type Match = { conversationId: string; title: string; updatedAt?: string; excerpt?: string };

export function ConversationSearchModal({
  open,
  onOpenChange,
  repository,
  onSelect,
  initialMatches = [],
  initialQuery = "",
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  repository: { repositoryId: string; sourceSnapshotId: string; graphVersion: string } | null;
  onSelect?: (match: Match) => void;
  initialMatches?: Match[];
  initialQuery?: string;
}) {
  const [query, setQuery] = useState(initialQuery);
  const [matches, setMatches] = useState<Match[]>(initialMatches);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    if (!open) return;
    setQuery(initialQuery);
    setMatches(initialMatches);
    setError("");
  }, [open, initialMatches, initialQuery]);

  async function search(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const value = query.trim().toLowerCase();
    if (!value) {
      setMatches(initialMatches);
      setError("");
      return;
    }

    setBusy(true);
    setError("");
    try {
      if (!repository) {
        setMatches(initialMatches.filter((match) => `${match.title} ${match.excerpt ?? ""}`.toLowerCase().includes(value)));
        return;
      }
      const params = new URLSearchParams({ q: query.trim(), limit: "20", ...repository });
      const response = await fetch(`/api/conversations/search?${params}`, { cache: "no-store" });
      const payload = await response.json() as { matches?: unknown[]; error?: string };
      if (!response.ok) throw new Error(payload.error ?? "Search is unavailable.");

      const values = (payload.matches ?? []).flatMap((value) => {
        if (!value || typeof value !== "object") return [];
        const item = value as Record<string, unknown>;
        return typeof item.conversationId === "string" && typeof item.title === "string"
          ? [{ conversationId: item.conversationId, title: item.title, updatedAt: typeof item.updatedAt === "string" ? item.updatedAt : undefined, excerpt: typeof item.excerpt === "string" ? item.excerpt : undefined }]
          : [];
      });
      setMatches(values);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Search is unavailable.");
      setMatches([]);
    } finally {
      setBusy(false);
    }
  }

  return (
    <Modal open={open} onOpenChange={onOpenChange}>
      <ModalContent className="w-[min(calc(100vw-2rem),56rem)] gap-3 bg-[#071018] p-4 sm:p-5">
        <ModalHeader className="pr-12"><div className="flex items-center gap-3"><Search className="size-5 text-primary" /><div><ModalTitle className="text-xl">Search Conversations</ModalTitle><ModalDescription>Search saved conversations by title, summary and repository context.</ModalDescription></div><kbd className="ml-auto hidden rounded-md border border-border bg-surface px-2 py-1 text-xs sm:inline">Ctrl K</kbd></div></ModalHeader>

        <form onSubmit={search} className="grid gap-3">
          <div className="flex items-center gap-2 rounded-lg border border-primary bg-surface px-3 focus-within:ring-2 focus-within:ring-ring"><Search aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" /><label htmlFor="conversation-modal-search" className="sr-only">Search saved conversations</label><Input id="conversation-modal-search" autoFocus type="search" value={query} onChange={(event) => setQuery(event.target.value)} maxLength={200} placeholder="Search conversations..." className="border-0 bg-transparent px-0 shadow-none focus-visible:ring-0" />{query && <button type="button" aria-label="Clear conversation search" onClick={() => { setQuery(""); setMatches(initialMatches); }} className="grid size-7 place-items-center rounded-full text-muted-foreground hover:bg-surface-alt hover:text-foreground"><X className="size-4" /></button>}</div>
          <div className="flex flex-wrap items-center gap-2"><span className="inline-flex items-center gap-1 rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground">All ({matches.length})</span><span className="inline-flex items-center gap-1 rounded-md border border-border px-3 py-1.5 text-xs text-muted-foreground">Today</span><span className="inline-flex items-center gap-1 rounded-md border border-border px-3 py-1.5 text-xs text-muted-foreground">This Week</span><span className="inline-flex items-center gap-1 rounded-md border border-border px-3 py-1.5 text-xs text-muted-foreground"><SlidersHorizontal className="size-3" />Repository</span><Button type="submit" className="ml-auto min-h-8 px-3 text-xs" disabled={busy}>{busy && <LoaderCircle className="size-3 animate-spin" />} {busy ? "Searching" : "Search"}</Button></div>
        </form>

        {error && <p role="alert" className="rounded-lg border border-danger/40 bg-danger/10 p-3 text-sm text-danger">{error}</p>}
        {!repository && <p role="status" className="text-xs text-muted-foreground">Showing conversations already loaded in the workspace. Repository-specific search becomes available when a current snapshot is selected.</p>}

        <div className="grid max-h-[55vh] gap-3 overflow-y-auto rounded-lg border border-border bg-background/30 p-2" aria-live="polite">
          {matches.map((match) => (
            <button key={match.conversationId} type="button" onClick={() => { onSelect?.(match); onOpenChange(false); }} className="grid gap-1 rounded-lg border border-border bg-surface p-3 text-left transition-colors hover:border-primary/60 hover:bg-surface-alt focus-visible:ring-2 focus-visible:ring-ring"><span className="text-sm font-medium">{match.title}</span>{match.excerpt && <span className="line-clamp-2 text-xs text-muted-foreground">{match.excerpt}</span>}{match.updatedAt && <time dateTime={match.updatedAt} className="text-xs text-muted-foreground">{new Date(match.updatedAt).toLocaleString()}</time>}</button>
          ))}
          {query.trim() && !busy && !error && matches.length === 0 && <div className="grid min-h-40 place-items-center p-6 text-center"><span className="grid size-12 place-items-center rounded-full bg-primary/10 text-primary"><Search className="size-6" /></span><p className="mt-3 text-sm font-medium">No results found</p><p className="mt-1 text-xs text-muted-foreground">Try different keywords or remove a filter.</p></div>}
          {!query.trim() && matches.length === 0 && <div className="grid min-h-40 place-items-center p-6 text-center"><Search className="size-8 text-muted-foreground" /><p className="mt-2 text-sm font-medium">No conversations found</p><p className="mt-1 text-xs text-muted-foreground">Start a new investigation to create a saved conversation.</p></div>}
        </div>

        <div className="flex flex-wrap gap-4 border-t border-border pt-3 text-xs text-muted-foreground"><span><kbd className="rounded border border-border bg-surface px-1.5 py-0.5">↑↓</kbd> Navigate</span><span><kbd className="rounded border border-border bg-surface px-1.5 py-0.5">Enter</kbd> Open</span><span><kbd className="rounded border border-border bg-surface px-1.5 py-0.5">Esc</kbd> Close</span></div>
      </ModalContent>
    </Modal>
  );
}
