"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { BookOpen, BrainCircuit, FolderGit2, MessageSquarePlus, Search, Settings2, Trash2 } from "lucide-react";
import { ConversationSearchModal } from "@/components/chat/conversation-search-modal";
import { cn } from "@/lib/utils";

const destinations = [
  { href: "/repositories", label: "Repositories", icon: FolderGit2 },
  { href: "/settings", label: "Settings", icon: Settings2 },
  { href: "/help", label: "Help & Docs", icon: BookOpen },
];

type RepositoryNavItem = { repositoryId: string; name: string; readiness: string; sourceSnapshotId?: string; graphVersion?: string };
type RecentConversation = { conversationId: string; title: string; href: string; deleteHref: string; updatedAt?: string };

function conversationGroup(updatedAt?: string): "Today" | "Yesterday" | "This Week" | "Older" {
  if (!updatedAt) return "Older";
  const date = new Date(updatedAt);
  if (Number.isNaN(date.getTime())) return "Older";
  const now = new Date();
  const startOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  const startOfDate = new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
  const day = 86_400_000;
  const difference = startOfToday - startOfDate;
  if (difference === 0) return "Today";
  if (difference === day) return "Yesterday";
  if (difference > day && difference < 7 * day) return "This Week";
  return "Older";
}

export function Sidebar({ onNavigate }: { onNavigate?: () => void }) {
  const pathname = usePathname();
  const router = useRouter();
  const [repositories, setRepositories] = useState<RepositoryNavItem[]>([]);
  const [conversations, setConversations] = useState<RecentConversation[]>([]);
  const [conversationQuery, setConversationQuery] = useState("");
  const [conversationSearchState, setConversationSearchState] = useState<"loading" | "ready" | "unavailable">("loading");
  const [conversationRevision, setConversationRevision] = useState(0);
  const [deletingConversationId, setDeletingConversationId] = useState<string | null>(null);
  const [conversationActionError, setConversationActionError] = useState(false);
  const [conversationSearchOpen, setConversationSearchOpen] = useState(false);
  const conversationSearchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const refresh = () => setConversationRevision((revision) => revision + 1);
    window.addEventListener("codemind:conversations-changed", refresh);
    return () => window.removeEventListener("codemind:conversations-changed", refresh);
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    fetch("/api/repositories", { signal: controller.signal, cache: "no-store" })
      .then(async (response) => {
        if (!response.ok) return;
        const result = await response.json() as { repositories?: unknown[] };
        setRepositories((result.repositories ?? []).flatMap((item) => {
          if (!item || typeof item !== "object") return [];
          const value = item as Record<string, unknown>;
          return typeof value.repositoryId === "string" && typeof value.name === "string"
            ? [{ repositoryId: value.repositoryId, name: value.name, readiness: typeof value.readiness === "string" ? value.readiness : "unknown", sourceSnapshotId: typeof value.sourceSnapshotId === "string" ? value.sourceSnapshotId : undefined, graphVersion: typeof value.graphVersion === "string" ? value.graphVersion : undefined }]
            : [];
        }).slice(0, 25));
      }).catch(() => undefined);
    return () => controller.abort();
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setConversationSearchOpen(true);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  useEffect(() => {
    const controller = new AbortController();
    setConversationSearchState("loading");
    const availableRepositories = repositories.filter((item) => item.sourceSnapshotId && item.graphVersion).slice(0, 10);
    void Promise.all(availableRepositories.map(async (repository) => {
      const query = new URLSearchParams({ repositoryId: repository.repositoryId, sourceSnapshotId: repository.sourceSnapshotId!, graphVersion: repository.graphVersion! });
      const endpoint = conversationQuery.trim()
        ? `/api/conversations/search?${new URLSearchParams({ ...Object.fromEntries(query), q: conversationQuery.trim(), limit: "20" })}`
        : `/api/conversations?${query}`;
      try {
        const response = await fetch(endpoint, { signal: controller.signal, cache: "no-store" });
        if (!response.ok) return { available: false, conversations: [] as RecentConversation[] };
        const payload = await response.json() as { conversations?: unknown[]; matches?: unknown[] };
        const items = (payload.conversations ?? payload.matches ?? []).flatMap((entry) => {
          if (!entry || typeof entry !== "object") return [];
          const item = entry as Record<string, unknown>;
          if (typeof item.conversationId !== "string" || typeof item.title !== "string") return [];
          if (!conversationQuery.trim() && (!Array.isArray(item.messages) || item.messages.length === 0)) return [];
          return [{ conversationId: item.conversationId, title: item.title, href: `/conversations/${encodeURIComponent(item.conversationId)}?${query}`, deleteHref: `/api/conversations/${encodeURIComponent(item.conversationId)}?${query}`, updatedAt: typeof item.updatedAt === "string" ? item.updatedAt : undefined }];
        });
        return { available: true, conversations: items };
      } catch (error) {
        if (error instanceof DOMException && error.name === "AbortError") throw error;
        return { available: false, conversations: [] as RecentConversation[] };
      }
    })).then((groups) => {
      setConversations(groups.flatMap((group) => group.conversations).slice(0, conversationQuery.trim() ? 20 : 8));
      setConversationSearchState(availableRepositories.length > 0 && groups.every((group) => !group.available) ? "unavailable" : "ready");
    }).catch((error: unknown) => {
      if (error instanceof DOMException && error.name === "AbortError") return;
      setConversationSearchState("unavailable");
    });
    return () => controller.abort();
  }, [repositories, conversationQuery, pathname, conversationRevision]);

  const searchMatches = useMemo(() => conversations.map((conversation) => ({ conversationId: conversation.conversationId, title: conversation.title, updatedAt: conversation.updatedAt })), [conversations]);

  async function deleteConversation(conversation: RecentConversation) {
    if (!window.confirm(`Delete “${conversation.title}”? This removes its saved messages.`)) return;
    setDeletingConversationId(conversation.conversationId);
    setConversationActionError(false);
    try {
      const response = await fetch(conversation.deleteHref, { method: "DELETE" });
      if (!response.ok) throw new Error("delete failed");
      window.dispatchEvent(new Event("codemind:conversations-changed"));
      if (pathname === `/conversations/${conversation.conversationId}`) router.push("/");
    } catch {
      setConversationActionError(true);
    } finally {
      setDeletingConversationId(null);
    }
  }

  return (
    <aside className="flex h-full min-h-0 flex-col bg-sidebar text-sidebar-foreground">
      <div className="flex h-16 items-center gap-2 border-b border-sidebar-border px-5">
        <BrainCircuit aria-hidden="true" className="size-8 text-primary" />
        <Link href="/" onClick={onNavigate} className="text-lg font-semibold tracking-tight focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sidebar-ring">
          Code<span className="text-primary">Mind</span>
          <span className="sr-only"> home</span>
        </Link>
        <span className="ml-auto rounded-full border border-sidebar-border px-2 py-0.5 text-[10px] text-muted-foreground">v1.0</span>
      </div>

      <nav aria-label="Primary navigation" className="grid gap-2 px-3 pt-4">
        <Link href="/" onClick={onNavigate} aria-current={pathname === "/" ? "page" : undefined} className="flex min-h-10 items-center gap-3 rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sidebar-ring">
          <MessageSquarePlus aria-hidden="true" className="size-4" />New Chat
        </Link>
        <label htmlFor="sidebar-conversation-search" className="flex min-h-10 items-center gap-2 rounded-md px-3 text-sm text-muted-foreground transition-colors focus-within:bg-sidebar-accent focus-within:text-sidebar-foreground focus-within:ring-2 focus-within:ring-sidebar-ring">
          <Search aria-hidden="true" className="size-4 shrink-0" />
          <input id="sidebar-conversation-search" ref={conversationSearchRef} type="search" value={conversationQuery} onFocus={() => setConversationSearchOpen(true)} onChange={(event) => { setConversationQuery(event.target.value.slice(0, 200)); setConversationSearchOpen(true); }} placeholder="Search conversations..." aria-label="Search conversations" className="min-w-0 flex-1 bg-transparent outline-none placeholder:text-muted-foreground" />
          <kbd className="rounded border border-sidebar-border px-1.5 py-0.5 text-[10px]">⌘K</kbd>
        </label>
      </nav>

      <div className="mt-5 flex-1 overflow-y-auto px-3 pb-3">
        <p className="px-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">Repositories</p>
        {repositories.length === 0 ? <p className="px-2 py-4 text-sm text-muted-foreground">No workspace repositories are available.</p> : <ul className="mt-2 grid gap-1">{repositories.map((repository) => <li key={repository.repositoryId}><Link href={`/repositories/${encodeURIComponent(repository.repositoryId)}`} onClick={onNavigate} aria-current={pathname === `/repositories/${repository.repositoryId}` ? "page" : undefined} className="flex min-h-9 items-center gap-2 rounded-md px-2 text-sm text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sidebar-ring"><FolderGit2 aria-hidden="true" className="size-4 shrink-0" /><span className="min-w-0 flex-1 truncate">{repository.name}</span><span className="size-2 shrink-0 rounded-full bg-muted-foreground" title={`Readiness: ${repository.readiness}`} /></Link></li>)}</ul>}
        <div className="mt-6"><p className="px-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">{conversationQuery.trim() ? "Search results" : "Recent conversations"}</p>{conversationActionError && <p role="alert" className="px-2 py-2 text-xs text-destructive">Conversation could not be deleted.</p>}{conversationSearchState === "unavailable" ? <p role="status" className="px-2 py-3 text-xs text-muted-foreground">Conversation search is unavailable.</p> : conversations.length === 0 ? <p role={conversationSearchState === "loading" ? "status" : undefined} className="px-2 py-3 text-xs text-muted-foreground">{conversationSearchState === "loading" ? "Loading conversations…" : conversationQuery.trim() ? "No matching conversations." : "No saved conversations for available snapshots."}</p> : <div className="mt-2 grid gap-4">
          {(["Today", "Yesterday", "This Week", "Older"] as const).map((group) => {
            const items = conversations.filter((conversation) => conversationGroup(conversation.updatedAt) === group);
            if (!items.length) return null;
            return (
              <section key={group} aria-labelledby={`conversation-group-${group.toLowerCase().replace(" ", "-")}`}>
                <h3 id={`conversation-group-${group.toLowerCase().replace(" ", "-")}`} className="px-2 text-[11px] font-medium uppercase tracking-wide text-muted-foreground">{group}</h3>
                <ul className="mt-1 grid gap-1">
                  {items.map((conversation) => (
                    <li key={conversation.conversationId} className="group flex min-w-0 items-center gap-1 rounded-md hover:bg-sidebar-accent">
                      <Link href={conversation.href} onClick={onNavigate} aria-current={pathname === `/conversations/${conversation.conversationId}` ? "page" : undefined} className="min-h-9 min-w-0 flex-1 truncate rounded-md px-2 py-2 text-sm text-muted-foreground hover:text-sidebar-foreground focus-visible:ring-2 focus-visible:ring-sidebar-ring">{conversation.title}</Link>
                      <button type="button" aria-label={`Delete conversation ${conversation.title}`} title="Delete conversation" disabled={deletingConversationId === conversation.conversationId} onClick={() => void deleteConversation(conversation)} className="mr-1 inline-grid size-8 shrink-0 place-items-center rounded-md text-muted-foreground opacity-0 transition-opacity hover:bg-danger/10 hover:text-danger focus-visible:opacity-100 focus-visible:ring-2 focus-visible:ring-sidebar-ring group-hover:opacity-100 disabled:opacity-50"><Trash2 aria-hidden="true" className="size-4" /></button>
                    </li>
                  ))}
                </ul>
              </section>
            );
          })}
        </div>}</div>
      </div>

      <nav aria-label="Workspace navigation" className="grid gap-1 border-t border-sidebar-border px-3 py-3">
        {destinations.map(({ href, label, icon: Icon }) => {
          const active = pathname === href || pathname.startsWith(`${href}/`);
          return <Link key={href} href={href} onClick={onNavigate} aria-current={active ? "page" : undefined} className={cn(
            "flex min-h-10 items-center gap-3 rounded-lg px-3 text-sm text-muted-foreground transition-colors hover:bg-sidebar-accent hover:text-sidebar-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sidebar-ring",
            active && "bg-sidebar-accent text-sidebar-primary",
          )}>
            <Icon aria-hidden="true" className="size-4" />{label}
          </Link>;
        })}
      </nav>
      <ConversationSearchModal
        open={conversationSearchOpen}
        onOpenChange={(open) => setConversationSearchOpen(open)}
        repository={repositories.find((repository) => repository.sourceSnapshotId && repository.graphVersion) ? (() => { const repository = repositories.find((item) => item.sourceSnapshotId && item.graphVersion)!; return { repositoryId: repository.repositoryId, sourceSnapshotId: repository.sourceSnapshotId!, graphVersion: repository.graphVersion! }; })() : null}
        initialQuery={conversationQuery}
        initialMatches={searchMatches}
        onSelect={(match) => { const conversation = conversations.find((item) => item.conversationId === match.conversationId); if (conversation) { setConversationQuery(""); router.push(conversation.href); onNavigate?.(); } }}
      />
      <div className="border-t border-sidebar-border px-5 py-3 text-xs text-muted-foreground" aria-label="Current workspace">Current workspace · authenticated session</div>
    </aside>
  );
}
