"use client";

import { useState, type FormEvent } from "react";
import { LoaderCircle, Search } from "lucide-react";
import { Modal, ModalContent, ModalDescription, ModalHeader, ModalTitle } from "@/components/ui/modal";

type Match = { conversationId: string; title: string; updatedAt?: string; excerpt?: string };

export function ConversationSearchModal({
  open,
  onOpenChange,
  repository,
  onSelect,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  repository: { repositoryId: string; sourceSnapshotId: string; graphVersion: string } | null;
  onSelect?: (match: Match) => void;
}) {
  const [query, setQuery] = useState("");
  const [matches, setMatches] = useState<Match[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  async function search(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!repository || !query.trim()) return;
    setBusy(true);
    setError("");
    try {
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
    } finally { setBusy(false); }
  }

  return <Modal open={open} onOpenChange={onOpenChange}>
    <ModalContent>
      <ModalHeader><ModalTitle>Search conversations</ModalTitle><ModalDescription>Search saved conversation titles and summaries for the selected repository snapshot.</ModalDescription></ModalHeader>
      <form onSubmit={search} className="grid gap-3">
        <label className="flex min-h-11 items-center gap-2 rounded-lg border border-border bg-background px-3 focus-within:ring-2 focus-within:ring-ring"><Search aria-hidden="true" className="size-4 text-muted-foreground" /><span className="sr-only">Search saved conversations</span><input autoFocus type="search" value={query} onChange={(event) => setQuery(event.target.value)} maxLength={200} placeholder="Search conversations" className="min-w-0 flex-1 bg-transparent text-sm outline-none" /></label>
        <button type="submit" disabled={busy || !repository || !query.trim()} className="inline-flex min-h-10 items-center justify-center gap-2 rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50">{busy && <LoaderCircle aria-hidden="true" className="size-4 animate-spin motion-reduce:animate-none" />}{busy ? "Searching" : "Search"}</button>
      </form>
      {error && <p role="alert" className="text-sm text-danger">{error}</p>}
      {!repository && <p role="status" className="text-sm text-muted-foreground">Choose a repository with a current snapshot before searching.</p>}
      <ul className="grid max-h-72 gap-2 overflow-y-auto" aria-live="polite">{matches.map((match) => <li key={match.conversationId}><button type="button" onClick={() => { onSelect?.(match); onOpenChange(false); }} className="grid w-full gap-1 rounded-lg border border-border bg-surface p-3 text-left hover:bg-surface-alt focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"><span className="text-sm font-medium">{match.title}</span>{match.excerpt && <span className="line-clamp-2 text-xs text-muted-foreground">{match.excerpt}</span>}{match.updatedAt && <time dateTime={match.updatedAt} className="text-xs text-muted-foreground">{match.updatedAt}</time>}</button></li>)}</ul>
      {query.trim() && !busy && !error && matches.length === 0 && <p role="status" className="text-sm text-muted-foreground">No matching saved conversations.</p>}
    </ModalContent>
  </Modal>;
}
