import { LoaderCircle } from "lucide-react";
import { MessageRenderer } from "./message-renderer";

export type ChatMessage = {
  id: string;
  role: "user" | "assistant";
  content: string;
  evidence?: readonly { id: string; label: string; href?: string }[];
};

export function MessageList({
  messages,
  streaming = false,
}: {
  messages: readonly ChatMessage[];
  streaming?: boolean;
}) {
  return <ol aria-label="Conversation messages" aria-live="polite" className="grid content-start gap-5">
    {messages.slice(0, 500).map((message) => <li key={message.id}><MessageRenderer role={message.role} content={message.content} evidence={message.evidence} /></li>)}
    {streaming && <li role="status" className="flex items-center gap-2 pl-11 text-sm text-muted-foreground"><LoaderCircle aria-hidden="true" className="size-4 animate-spin motion-reduce:animate-none" />CodeMind is preparing an evidence-grounded response</li>}
  </ol>;
}
