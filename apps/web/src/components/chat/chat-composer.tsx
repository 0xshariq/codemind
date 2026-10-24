"use client";

import { useState, type FormEvent, type KeyboardEvent } from "react";
import { ArrowUp, LoaderCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";

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
    try {
      await onSubmit(question.trim());
      setQuestion("");
    } finally {
      setBusy(false);
    }
  }

  function keyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
      event.preventDefault();
      event.currentTarget.form?.requestSubmit();
    }
  }

  return (
    <form onSubmit={submit} className="grid gap-3 rounded-xl border border-border bg-surface p-3 shadow-sm focus-within:border-primary/70 sm:p-4">
      {suggestions.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {suggestions.slice(0, 6).map((suggestion) => (
            <button
              key={suggestion.label}
              type="button"
              onClick={() => setQuestion(suggestion.prompt)}
              disabled={disabled || busy}
              className="min-h-8 rounded-full border border-border px-3 text-xs text-muted-foreground transition-colors hover:bg-surface-alt hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50"
            >
              {suggestion.label}
            </button>
          ))}
        </div>
      )}

      <label htmlFor="chat-composer-question" className="sr-only">
        Ask about {repositoryName || "the repository"}
      </label>
      <Textarea
        id="chat-composer-question"
        value={question}
        onChange={(event) => setQuestion(event.target.value)}
        onKeyDown={keyDown}
        maxLength={4000}
        rows={4}
        disabled={disabled || busy}
        placeholder="Ask CodeMind anything about your codebase..."
        className="min-h-28 resize-none border-0 bg-transparent px-1 py-1 shadow-none focus-visible:border-0 focus-visible:ring-0"
      />

      <div className="flex flex-wrap items-center justify-between gap-3 border-t border-border pt-3">
        <p className="text-xs text-muted-foreground">
          {repositoryName
            ? `Using ${repositoryName}. Shift+Enter for a new line.`
            : "Select a repository to begin."}
        </p>
        <Button type="submit" disabled={disabled || busy || !question.trim()} className="min-w-28">
          {busy ? (
            <>
              <LoaderCircle aria-hidden="true" className="size-4 animate-spin motion-reduce:animate-none" />
              Sending
            </>
          ) : (
            <>
              Investigate
              <ArrowUp aria-hidden="true" className="size-4" />
            </>
          )}
        </Button>
      </div>

      {error && (
        <p role="alert" className="border-t border-border pt-3 text-sm text-danger">
          {error}
        </p>
      )}
    </form>
  );
}
