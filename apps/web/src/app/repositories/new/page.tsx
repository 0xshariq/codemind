"use client";

import { useState, type FormEvent } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { BookOpen, Check, CircleAlert, Folder, GitBranch, LoaderCircle, Server } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

type SourceKind = "github" | "local" | "example";
type Outcome = "accepted" | "indexing" | "completed" | "partial" | "unsupported" | "failed";
type Submission = { outcome: Outcome; repositoryId?: string; message?: string; limitations?: string[] };

const sourceOptions: Array<{ id: SourceKind; title: string; description: string; icon: typeof GitBranch }> = [
  { id: "github", title: "GitHub repository", description: "Import a public repository or a private repository accessible with the configured token.", icon: GitBranch },
  { id: "local", title: "Local repository", description: "Use a repository path available to the Next.js server.", icon: Folder },
  { id: "example", title: "Example repository", description: "Use an example source configured for this workspace.", icon: BookOpen },
];

function validateSource(kind: SourceKind, source: string) {
  if (kind === "example") return "";
  if (!source.trim()) return kind === "github" ? "Enter a GitHub repository URL." : "Enter a local repository path.";
  if (kind === "local") return source.trim().length <= 1024 ? "" : "The local path is too long.";
  try {
    const url = new URL(source);
    if (url.protocol !== "https:" || url.hostname !== "github.com" || url.username || url.password) return "Enter an HTTPS URL on github.com.";
    const segments = url.pathname.split("/").filter(Boolean);
    if (segments.length !== 2 || segments.some((segment) => segment === "." || segment === "..")) return "Enter a repository URL in the form https://github.com/owner/repository.";
    return "";
  } catch {
    return "Enter a valid HTTPS GitHub repository URL.";
  }
}

function Stepper({ active }: { active: number }) {
  const steps = ["Type", "Configure", "Analyze", "Complete"];
  return <ol aria-label="Repository setup progress" className="grid grid-cols-4 gap-2">
    {steps.map((step, index) => <li key={step} aria-current={active === index + 1 ? "step" : undefined} className="flex items-center gap-2 text-xs sm:text-sm">
      <span className={`grid size-7 shrink-0 place-items-center rounded-full border ${active > index + 1 ? "border-success bg-success text-background" : active === index + 1 ? "border-primary bg-primary text-primary-foreground" : "border-border bg-surface-alt text-muted-foreground"}`}>
        {active > index + 1 ? <Check aria-hidden="true" className="size-4" /> : index + 1}
      </span><span className={active === index + 1 ? "text-foreground" : "text-muted-foreground"}>{step}</span>
    </li>)}
  </ol>;
}

export default function NewRepositoryPage() {
  const router = useRouter();
  const [kind, setKind] = useState<SourceKind | null>(null);
  const [source, setSource] = useState("");
  const [name, setName] = useState("");
  const [submission, setSubmission] = useState<Submission | null>(null);
  const [error, setError] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [showErrors, setShowErrors] = useState(false);

  const sourceError = kind ? validateSource(kind, source) : "Choose a repository source type.";
  const nameError = kind === "example" && !name.trim() ? "Enter the configured example repository name."
    : kind && !name.trim() ? "Enter a repository name." : "";
  const step = submission ? submission.outcome === "completed" || submission.outcome === "partial" ? 4 : 3 : kind ? 2 : 1;

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setShowErrors(true);
    setError("");
    if (!kind || sourceError || nameError) return;
    setSubmitting(true);
    setSubmission(null);
    try {
      const response = await fetch("/api/repositories", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ sourceType: kind, source: source.trim(), name: name.trim() || undefined }),
      });
      const payload: unknown = await response.json().catch(() => null);
      const value = payload && typeof payload === "object" && !Array.isArray(payload)
        ? payload as Partial<Submission> & { error?: unknown }
        : {};
      if (!response.ok) {
        const apiMessage = typeof value.error === "string" ? value.error
          : typeof value.message === "string" ? value.message : "Repository preparation is unavailable.";
        throw new Error(response.status === 401
          ? `${apiMessage} Enable the local development session or sign in, then try again.`
          : apiMessage);
      }
      if (!["accepted", "indexing", "completed", "partial", "unsupported", "failed"].includes(value.outcome ?? "")) throw new Error("Repository preparation returned an invalid state.");
      const next = value as Submission;
      setSubmission(next);
      if (next.repositoryId && ["completed", "partial"].includes(next.outcome)) router.push(`/repositories/${encodeURIComponent(next.repositoryId)}`);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Repository preparation is unavailable.");
    } finally {
      setSubmitting(false);
    }
  }

  const activeOption = sourceOptions.find((option) => option.id === kind);

  return (
    <main className="mx-auto grid w-full max-w-6xl gap-6 p-4 sm:p-6 lg:p-8">
      <header><p className="text-sm text-muted-foreground">REPOSITORIES</p><h1 className="mt-1 text-3xl font-semibold tracking-tight">Add a repository</h1><p className="mt-2 text-sm text-muted-foreground">Connect a source for CodeMind to prepare and analyze.</p></header>
      <Card>
        <CardHeader><CardTitle>Repository setup</CardTitle><CardDescription>Source, configuration and indexing states come from the preparation service.</CardDescription><Stepper active={step} /></CardHeader>
        <CardContent className="grid gap-6">
          <fieldset className="grid gap-3" disabled={submitting}>
            <legend className="mb-2 text-sm font-medium">1. Choose repository type</legend>
            <div className="grid gap-3 md:grid-cols-3">
              {sourceOptions.map(({ id, title, description, icon: Icon }) => <label key={id} className={`flex min-h-36 cursor-pointer flex-col gap-3 rounded-xl border p-4 transition-colors focus-within:ring-2 focus-within:ring-ring ${kind === id ? "border-primary bg-primary/5" : "border-border bg-surface hover:bg-surface-alt"}`}>
                <span className="flex items-center justify-between"><Icon aria-hidden="true" className="size-6 text-primary" /><input type="radio" name="repository-source" value={id} checked={kind === id} onChange={() => { setKind(id); setSubmission(null); setShowErrors(false); }} className="size-4 accent-primary" /></span>
                <span className="font-medium">{title}</span><span className="text-sm text-muted-foreground">{description}</span>
              </label>)}
            </div>
            {showErrors && !kind && <p role="alert" className="text-sm text-danger">Choose a repository source type.</p>}
          </fieldset>

          {activeOption && <form onSubmit={submit} className="grid gap-5 border-t border-border pt-5">
            <div><h2 className="text-lg font-semibold">2. Configure repository</h2><p className="mt-1 text-sm text-muted-foreground">Only the selected source details are sent for validation.</p></div>
            {kind !== "example" && <div className="grid gap-2">
              <label htmlFor="repository-source" className="text-sm font-medium">{kind === "github" ? "GitHub repository URL" : "Local repository path"}</label>
              <input id="repository-source" value={source} onChange={(event) => setSource(event.target.value)} placeholder={kind === "github" ? "https://github.com/owner/repository" : "/path/to/repository"} maxLength={1024} autoComplete="off" aria-invalid={showErrors && Boolean(sourceError)} aria-describedby={showErrors && sourceError ? "repository-source-error" : "repository-source-help"} className="min-h-11 rounded-md border border-border bg-surface px-3 text-sm outline-none placeholder:text-muted-foreground focus:border-primary focus-visible:ring-2 focus-visible:ring-ring aria-invalid:border-danger" />
              {showErrors && sourceError ? <p id="repository-source-error" role="alert" className="text-sm text-danger">{sourceError}</p> : <p id="repository-source-help" className="text-xs text-muted-foreground">{kind === "github" ? "Only HTTPS github.com owner/repository URLs are accepted by this form." : "The server must validate this path against its configured repository roots."}</p>}
            </div>}
            <div className="grid gap-2"><label htmlFor="repository-name" className="text-sm font-medium">{kind === "example" ? "Configured example name" : "Repository name"}</label><input id="repository-name" required aria-invalid={showErrors && Boolean(nameError)} aria-describedby={showErrors && nameError ? "repository-name-error" : undefined} value={name} onChange={(event) => setName(event.target.value)} maxLength={120} className="min-h-11 rounded-md border border-border bg-surface px-3 text-sm outline-none focus:border-primary focus-visible:ring-2 focus-visible:ring-ring aria-invalid:border-danger" />{showErrors && nameError && <p id="repository-name-error" role="alert" className="text-sm text-danger">{nameError}</p>}</div>

            {error && <div role="alert" className="flex items-start gap-2 rounded-lg border border-danger/50 bg-danger/10 p-3 text-sm"><CircleAlert aria-hidden="true" className="mt-0.5 size-4 shrink-0 text-danger" /><span>{error}</span></div>}
            {submission && <div role="status" className="grid gap-2 rounded-lg border border-border bg-surface-alt p-4"><div className="flex flex-wrap items-center gap-2"><Badge variant={submission.outcome === "completed" ? "default" : submission.outcome === "failed" ? "destructive" : "secondary"}>{submission.outcome}</Badge><span className="text-sm">{submission.message ?? "Repository preparation status returned by the service."}</span></div>{submission.limitations?.map((limitation) => <p key={limitation} className="text-sm text-muted-foreground">{limitation}</p>)}{submission.repositoryId && <Link href={`/repositories/${encodeURIComponent(submission.repositoryId)}`} className="text-sm text-primary hover:underline">Open repository details</Link>}</div>}

            <div className="flex flex-wrap items-center justify-between gap-3 border-t border-border pt-4">
              <Link href="/repositories" className="inline-flex min-h-10 items-center rounded-md border border-border px-4 text-sm hover:bg-surface-alt focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">Cancel</Link>
              <button type="submit" disabled={submitting} className="inline-flex min-h-11 items-center gap-2 rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:bg-primary-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-wait disabled:opacity-60">
                {submitting ? <><LoaderCircle aria-hidden="true" className="size-4 animate-spin" />Preparing repository</> : <><Server aria-hidden="true" className="size-4" />Start analysis</>}
              </button>
            </div>
          </form>}
        </CardContent>
      </Card>
      <p className="text-xs text-muted-foreground">Supported source types are developer-supplied local repositories, bounded public GitHub imports and examples configured for this workspace.</p>
    </main>
  );
}
