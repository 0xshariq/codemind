import Link from "next/link";
import { MessageSquarePlus, Search } from "lucide-react";
import { EmptyState } from "@/components/common/empty-state";
import { ErrorState } from "@/components/common/error-state";
import { LoadingState } from "@/components/common/loading-state";
import { Button } from "@/components/ui/button";

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
  if (state === "error" || state === "unavailable") {
    return (
      <ErrorState
        title="Conversation history unavailable"
        description="Saved conversations could not be retrieved for the active workspace."
        onRetry={onRetry}
      />
    );
  }

  return (
    <section className="grid gap-4" aria-label="Conversation history">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-base font-semibold">Recent conversations</h2>
          <p className="mt-1 text-sm text-muted-foreground">Continue an investigation or start a new one.</p>
        </div>
        <div className="flex gap-2">
          <Button type="button" variant="outline" size="sm" onClick={onSearch} disabled={!onSearch}>
            <Search aria-hidden="true" />
            Search
          </Button>
          <Link href="/" className="inline-flex h-8 items-center justify-center gap-1.5 rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary-hover focus-visible:ring-2 focus-visible:ring-ring">
            <MessageSquarePlus aria-hidden="true" className="size-4" />
            New chat
          </Link>
        </div>
      </div>

      {conversations.length === 0 ? (
        <EmptyState title="No saved conversations" description="Conversations for the selected repository will appear here." />
      ) : (
        <ul className="grid gap-1">
          {conversations.slice(0, 100).map((item) => (
            <li key={item.conversationId}>
              <button
                type="button"
                onClick={() => onOpen?.(item.conversationId)}
                disabled={!onOpen}
                className="grid min-h-12 w-full gap-1 rounded-md border border-transparent px-3 py-2 text-left transition-colors hover:border-border hover:bg-surface-alt focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-default"
              >
                <span className="truncate text-sm font-medium">{item.title}</span>
                {item.updatedAt && <time dateTime={item.updatedAt} className="text-xs text-muted-foreground">{item.updatedAt}</time>}
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
