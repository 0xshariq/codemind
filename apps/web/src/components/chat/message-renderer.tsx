import { Bot, UserRound } from "lucide-react";

function safeEvidenceHref(value: string | undefined) {
  if (!value || value.startsWith("//") || !(value.startsWith("/") || value.startsWith("#"))) return undefined;
  return value;
}

export function MessageRenderer({
  role,
  content,
  evidence,
}: {
  role: "user" | "assistant";
  content: string;
  evidence?: readonly { id: string; label: string; href?: string }[];
}) {
  const parts = content.slice(0, 40_000).split(/\n{2,}/).slice(0, 200);

  return (
    <article className={`grid max-w-3xl grid-cols-[2rem_minmax(0,1fr)] gap-3 ${role === "user" ? "ml-auto" : "mr-auto"}`}>
      <span
        aria-hidden="true"
        className={`grid size-8 place-items-center rounded-full border ${
          role === "user"
            ? "border-border bg-surface-alt text-foreground"
            : "border-primary/20 bg-primary/10 text-primary"
        }`}
      >
        {role === "user" ? <UserRound className="size-4" /> : <Bot className="size-4" />}
      </span>

      <div className={`min-w-0 rounded-lg border px-4 py-3 ${
        role === "user" ? "border-border bg-surface-alt" : "border-border bg-surface"
      }`}>
        <p className="mb-2 text-xs font-medium text-muted-foreground">{role === "user" ? "You" : "CodeMind"}</p>
        <div className="grid gap-3">
          {parts.map((part, index) => (
            <p key={`${index}-${part.slice(0, 24)}`} className="whitespace-pre-wrap break-words text-sm leading-6">
              {part}
            </p>
          ))}
        </div>

        {evidence?.length ? (
          <ul aria-label="Referenced evidence" className="mt-3 grid gap-1 border-t border-border pt-3">
            {evidence.slice(0, 20).map((item) => {
              const href = safeEvidenceHref(item.href);
              return (
                <li key={item.id}>
                  {href ? (
                    <a href={href} className="break-all text-xs text-primary underline underline-offset-2 focus-visible:ring-2 focus-visible:ring-ring">
                      {item.label}
                    </a>
                  ) : (
                    <span className="break-all text-xs text-muted-foreground">{item.label}</span>
                  )}
                </li>
              );
            })}
          </ul>
        ) : null}
      </div>
    </article>
  );
}
