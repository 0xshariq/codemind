import Link from "next/link";
import { MessageSquarePlus, Search } from "lucide-react";
import { EmptyState } from "@/components/common/empty-state";
import { ErrorState } from "@/components/common/error-state";
import { LoadingState } from "@/components/common/loading-state";

export type ConversationListItem = { conversationId: string; title: string; updatedAt?: string };

export function ConversationList({
  conversations,
  state = "ready",
  onSearch,
  onOpen,
  onRetry,
}: {
  conversations: readonly ConversationListItem[];
  state?: "loading" | "ready" | "error" | "unavailable";
  onSearch?: () => void;
  onOpen?: (conversationId: string) => void;
  onRetry?: () => void;
}) {
  if (state === "loading") return <LoadingState label="Loading conversations" />;
  if (state === "error" || state === "unavailable") return <ErrorState title="Conversation history unavailable" description="Saved conversations could not be retrieved for the active workspace." onRetry={onRetry} />;
  return <section className="grid gap-3" aria-label="Conversation history">
    <div className="flex flex-wrap items-center justify-between gap-2"><h2 className="text-sm font-semibold">Recent conversations</h2><div className="flex gap-2"><button type="button" onClick={onSearch} disabled={!onSearch} className="inline-flex min-h-9 items-center gap-2 rounded-md border border-border px-3 text-xs hover:bg-surface-alt focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"><Search aria-hidden="true" className="size-3.5" />Search</button><Link href="/" className="inline-flex min-h-9 items-center gap-2 rounded-md bg-primary px-3 text-xs font-medium text-primary-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"><MessageSquarePlus aria-hidden="true" className="size-3.5" />New chat</Link></div></div>
    {conversations.length === 0 ? <EmptyState title="No saved conversations" description="Conversations for the selected repository will appear here." /> : <ul className="grid gap-1">{conversations.slice(0, 100).map((item) => <li key={item.conversationId}><button type="button" onClick={() => onOpen?.(item.conversationId)} disabled={!onOpen} className="grid min-h-11 w-full gap-1 rounded-md px-3 py-2 text-left hover:bg-surface-alt focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-default"><span className="truncate text-sm">{item.title}</span>{item.updatedAt && <time dateTime={item.updatedAt} className="text-xs text-muted-foreground">{item.updatedAt}</time>}</button></li>)}</ul>}
  </section>;
}
