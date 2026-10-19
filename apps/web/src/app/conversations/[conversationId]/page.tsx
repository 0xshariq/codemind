"use client";

import { use, useEffect, useState } from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { ArrowLeft } from "lucide-react";
import { ChatPanel } from "@/components/chat/chat-panel";
import type { ChatMessage } from "@/components/chat/message-list";

type Conversation = { title: string; context: { repositoryId: string }; messages: ChatMessage[] };
type Repository = { name: string };

export default function ConversationPage({ params }: PageProps<"/conversations/[conversationId]">) {
  const { conversationId: encodedConversationId } = use(params);
  let conversationId = encodedConversationId;
  try { conversationId = decodeURIComponent(encodedConversationId); } catch { /* invalid IDs are rejected by the API */ }
  const search = useSearchParams();
  const [conversation, setConversation] = useState<Conversation | null>(null);
  const [repositoryName, setRepositoryName] = useState("");
  const [state, setState] = useState<"loading" | "ready" | "unavailable">("loading");
  const repositoryId = search.get("repositoryId") ?? "";
  const sourceSnapshotId = search.get("sourceSnapshotId") ?? "";
  const graphVersion = search.get("graphVersion") ?? "";

  useEffect(() => {
    const controller = new AbortController();
    const query = new URLSearchParams({ repositoryId, sourceSnapshotId, graphVersion });
    Promise.all([
      fetch(`/api/conversations/${encodeURIComponent(conversationId)}?${query}`, { signal: controller.signal, cache: "no-store" }),
      fetch(`/api/repositories/${encodeURIComponent(repositoryId)}`, { signal: controller.signal, cache: "no-store" }),
    ])
      .then(async ([conversationResponse, repositoryResponse]) => {
        if (!conversationResponse.ok) throw new Error("unavailable");
        const payload = await conversationResponse.json() as { conversation?: Conversation };
        if (!payload.conversation || payload.conversation.context.repositoryId !== repositoryId) throw new Error("unavailable");
        setConversation(payload.conversation);
        if (repositoryResponse.ok) {
          const repositoryPayload = await repositoryResponse.json() as { repository?: Repository };
          setRepositoryName(repositoryPayload.repository?.name ?? "");
        }
        setState("ready");
      })
      .catch((error: unknown) => {
        if (error instanceof DOMException && error.name === "AbortError") return;
        setState("unavailable");
      });
    return () => controller.abort();
  }, [conversationId, repositoryId, sourceSnapshotId, graphVersion]);

  useEffect(() => {
    const refreshConversation = () => {
      const query = new URLSearchParams({ repositoryId, sourceSnapshotId, graphVersion });
      void fetch(`/api/conversations/${encodeURIComponent(conversationId)}?${query}`, { cache: "no-store" })
        .then(async (response) => {
          if (!response.ok) return;
          const payload = await response.json() as { conversation?: Conversation };
          if (payload.conversation?.context.repositoryId === repositoryId) setConversation(payload.conversation);
        })
        .catch(() => undefined);
    };
    window.addEventListener("codemind:conversations-changed", refreshConversation);
    return () => window.removeEventListener("codemind:conversations-changed", refreshConversation);
  }, [conversationId, repositoryId, sourceSnapshotId, graphVersion]);

  if (state === "loading") return <main className="p-6" role="status">Loading saved conversation…</main>;
  if (state !== "ready" || !conversation) return <main className="p-6" role="alert">Conversation is unavailable for this repository snapshot.</main>;
  return <main className="mx-auto grid w-full max-w-7xl gap-5 p-4 sm:p-6"><header className="grid gap-3 border-b border-border pb-4"><Link href={`/repositories/${encodeURIComponent(repositoryId)}`} className="inline-flex w-fit items-center gap-2 text-sm text-muted-foreground hover:text-foreground"><ArrowLeft aria-hidden="true" className="size-4" />{repositoryName || "Repository"}</Link><div><p className="text-sm text-muted-foreground">Repository investigation · {repositoryName || repositoryId}</p><h1 className="text-xl font-semibold">{conversation.title}</h1><p className="mt-1 break-all font-mono text-xs text-muted-foreground">Conversation {conversationId}</p></div></header><ChatPanel repositoryId={repositoryId} repositoryName={repositoryName} conversationId={conversationId} initialMessages={conversation.messages} /></main>;
}
