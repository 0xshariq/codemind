"use client";

import { useState, type FormEvent, type KeyboardEvent } from "react";
import { ArrowUp, LoaderCircle } from "lucide-react";

export function ChatComposer({
  repositoryName,
  onSubmit,
  disabled = false,
  error,
  suggestions = [],
}: {
  repositoryName?: string;
  onSubmit: (question: string) => Promise<void> | void;
  disabled?: boolean;
  error?: string;
  suggestions?: readonly { label: string; prompt: string }[];
}) {
  const [question, setQuestion] = useState("");
  const [busy, setBusy] = useState(false);
  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (disabled || busy || !question.trim()) return;
    setBusy(true);
    try { await onSubmit(question.trim()); setQuestion(""); }
    finally { setBusy(false); }
  }
  function keyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
      event.preventDefault();
      event.currentTarget.form?.requestSubmit();
    }
  }
  return <form onSubmit={submit} className="grid gap-3 rounded-xl border border-border bg-surface p-3 focus-within:border-primary sm:p-4">
    {suggestions.length > 0 && <div className="flex flex-wrap gap-2">{suggestions.slice(0, 6).map((suggestion) => <button key={suggestion.label} type="button" onClick={() => setQuestion(suggestion.prompt)} disabled={disabled || busy} className="min-h-8 rounded-full border border-border px-3 text-xs text-muted-foreground hover:bg-surface-alt focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">{suggestion.label}</button>)}</div>}
    <label htmlFor="chat-composer-question" className="sr-only">Ask about {repositoryName || "the repository"}</label><textarea id="chat-composer-question" value={question} onChange={(event) => setQuestion(event.target.value)} onKeyDown={keyDown} maxLength={4000} rows={3} disabled={disabled || busy} placeholder="Ask about code relationships, impact, or tests…" className="max-h-56 min-h-20 resize-y bg-transparent px-2 py-1 text-sm outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-60" />
    <div className="flex flex-wrap items-center justify-between gap-2 border-t border-border pt-3"><p className="text-xs text-muted-foreground">{repositoryName ? `Using ${repositoryName}. Ask in plain language; a file or module name is optional. Shift+Enter for a new line.` : "Select a repository to begin."}</p><button type="submit" disabled={disabled || busy || !question.trim()} className="inline-flex min-h-10 items-center gap-2 rounded-lg bg-primary px-4 text-sm font-medium text-primary-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50">{busy ? <><LoaderCircle aria-hidden="true" className="size-4 animate-spin motion-reduce:animate-none" />Sending</> : <>Investigate <ArrowUp aria-hidden="true" className="size-4" /></>}</button></div>
    {error && <p role="alert" className="text-sm text-danger">{error}</p>}
  </form>;
}
