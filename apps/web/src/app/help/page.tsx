import { Suspense } from "react";
import Link from "next/link";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

const helpSections = [
  {
    id: "getting-started",
    title: "Getting started",
    summary: "Choose an available repository, ask about a change, feature or error, then inspect the structured result and its evidence. Naming a file or module can narrow the focus.",
    keywords: "repository question analysis evidence results",
  },
  {
    id: "what-is-codemind",
    title: "What is CodeMind?",
    summary: "CodeMind investigates code relationships and explains how a selected change may affect connected code and tests.",
    keywords: "codebase investigation impact agent",
  },
  {
    id: "how-it-works",
    title: "How analysis works",
    summary: "The Agent Runtime uses approved semantic tools. The Rust engine owns canonical code, test and change facts; the language model explains bounded evidence.",
    keywords: "agent runtime rust llm semantic tools architecture",
  },
  {
    id: "analysis-engines",
    title: "Analysis engines",
    summary: "CoIE builds code intelligence, TIE discovers test intelligence, and ChIE derives change impact, qualitative risk and evidence-backed paths.",
    keywords: "CoIE TIE ChIE code test change engine",
  },
  {
    id: "graph-evidence",
    title: "Graph and evidence",
    summary: "Impact paths preserve relationship direction and type, repository snapshot context and source evidence. Partial or truncated results retain their limitations.",
    keywords: "graph relationship path snapshot source evidence partial truncated",
  },
  {
    id: "shortcuts",
    title: "Keyboard shortcuts",
    summary: "Use Tab and Shift+Tab to move through controls, Enter or Space to activate a focused control, and Escape to close an open dialog.",
    keywords: "keyboard shortcuts accessibility navigation",
  },
  {
    id: "troubleshooting",
    title: "Troubleshooting",
    summary: "If a repository or engine is unavailable, check its readiness and service configuration. An unavailable result is not a successful empty analysis.",
    keywords: "troubleshooting unavailable failed empty service repository readiness",
  },
  {
    id: "faq",
    title: "Frequently asked questions",
    summary: "Risk, impact, completeness and evidence freshness are separate result dimensions. A test candidate is a recommendation, not proof that a test ran.",
    keywords: "FAQ risk impact completeness freshness tests candidate",
  },
];

export default function HelpPage(props: PageProps<"/help">) {
  return (
    <Suspense fallback={<main className="mx-auto grid w-full max-w-7xl gap-8 p-4 sm:p-6 lg:p-8" aria-busy="true"><p role="status" className="text-sm text-muted-foreground">Loading help topics…</p></main>}>
      <HelpPageContent searchParams={props.searchParams} />
    </Suspense>
  );
}

async function HelpPageContent({ searchParams }: { searchParams: PageProps<"/help">["searchParams"] }) {
  const query = ((await searchParams).q ?? "").toString().trim().toLocaleLowerCase();
  const visibleSections = query
    ? helpSections.filter((section) => `${section.title} ${section.summary} ${section.keywords}`.toLocaleLowerCase().includes(query))
    : helpSections;

  return (
    <main className="mx-auto grid w-full max-w-7xl gap-8 p-4 sm:p-6 lg:p-8">
      <header className="grid gap-5 border-b border-border pb-6 lg:grid-cols-[1fr_auto] lg:items-end">
        <div className="grid gap-2">
          <p className="text-sm font-medium text-primary">CODEMIND GUIDE</p>
          <h1 className="text-3xl font-semibold tracking-tight sm:text-4xl">Help &amp; Docs</h1>
          <p className="max-w-2xl text-sm text-muted-foreground sm:text-base">Ask about a change, feature or error in plain language. Naming a file or module can narrow the focus, and every result keeps its supporting evidence visible.</p>
        </div>
        <form action="/help" role="search" className="flex w-full items-center gap-2 rounded-lg border border-border bg-surface px-3 py-2 focus-within:border-primary lg:w-80">
          <label htmlFor="help-search" className="sr-only">Search help topics</label>
          <input id="help-search" name="q" type="search" defaultValue={query} placeholder="Search documentation" className="min-w-0 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground" />
          <button type="submit" className="rounded-md bg-surface-alt px-2 py-1 text-xs text-muted-foreground hover:text-foreground">Search</button>
        </form>
      </header>

      {visibleSections.length === 0 ? <section aria-live="polite" className="rounded-xl border border-border bg-surface p-6">
        <h2 className="font-semibold">No matching help topics</h2>
        <p className="mt-2 text-sm text-muted-foreground">Try another search, or browse all documentation topics.</p>
        <Link href="/help" className="mt-4 inline-flex text-sm text-primary underline-offset-4 hover:underline">Show all topics</Link>
      </section> : <Tabs defaultValue={visibleSections[0].id} className="flex w-full flex-col gap-5">
        <TabsList aria-label="Help topics" className="!grid h-auto w-full grid-cols-2 auto-rows-fr items-stretch gap-2 rounded-xl bg-transparent p-0 sm:grid-cols-3 xl:grid-cols-4">
          {visibleSections.map((section) => <TabsTrigger key={section.id} value={section.id} className="group !h-auto min-h-20 self-stretch justify-between whitespace-normal rounded-xl border border-border bg-surface p-3 text-left data-active:border-primary data-active:bg-primary/10 data-active:text-foreground">
            <span className="grid gap-1"><span className="font-medium">{section.title}</span><span className="line-clamp-2 text-xs text-muted-foreground">{section.summary}</span></span>
            <span aria-hidden="true" className="ml-2 shrink-0 text-primary">→</span>
          </TabsTrigger>)}
        </TabsList>
        <div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_280px]">
        <div className="min-w-0">
          {visibleSections.map((section) => <TabsContent key={section.id} value={section.id}>
            <section id={section.id} className="rounded-xl border border-border bg-surface p-5 sm:p-6">
            <h2 className="text-xl font-semibold">{section.title}</h2>
            <p className="mt-2 text-sm leading-6 text-muted-foreground">{section.summary}</p>
            {section.id === "getting-started" && <ol className="mt-5 grid gap-3 sm:grid-cols-3">
              <li className="rounded-lg border border-border p-4"><span className="text-sm font-semibold text-primary">1. Choose a repository</span><p className="mt-2 text-sm text-muted-foreground">Open Repositories and select a repository that is available to your workspace.</p><Link href="/repositories" className="mt-3 inline-block text-sm text-primary hover:underline">Browse repositories</Link></li>
              <li className="rounded-lg border border-border p-4"><span className="text-sm font-semibold text-primary">2. Ask a question</span><p className="mt-2 text-sm text-muted-foreground">Describe the change, feature or error. CodeMind can select a relevant source file; naming a file or module is optional.</p><Link href="/" className="mt-3 inline-block text-sm text-primary hover:underline">Start an investigation</Link></li>
              <li className="rounded-lg border border-border p-4"><span className="text-sm font-semibold text-primary">3. Review the result</span><p className="mt-2 text-sm text-muted-foreground">Read impact, risk, test candidates, evidence and any stated limitations.</p></li>
            </ol>}
            {section.id === "analysis-engines" && <dl className="mt-5 grid gap-3 sm:grid-cols-3">
              <div className="rounded-lg border border-border p-4"><dt className="font-mono font-semibold text-primary">CoIE</dt><dd className="mt-2 text-sm text-muted-foreground">Code Intelligence Engine: code entities and relationships.</dd></div>
              <div className="rounded-lg border border-border p-4"><dt className="font-mono font-semibold text-accent">TIE</dt><dd className="mt-2 text-sm text-muted-foreground">Test Intelligence Engine: test declarations and supported mappings.</dd></div>
              <div className="rounded-lg border border-border p-4"><dt className="font-mono font-semibold text-success">ChIE</dt><dd className="mt-2 text-sm text-muted-foreground">Change Intelligence Engine: impact, risk and evidence-backed paths.</dd></div>
            </dl>}
            {section.id === "how-it-works" && <div className="mt-5 flex flex-wrap items-center gap-2 text-xs sm:text-sm"><span className="rounded-md border border-border px-3 py-2">Question</span><span aria-hidden="true" className="text-primary">→</span><span className="rounded-md border border-border px-3 py-2">LLM Planning</span><span aria-hidden="true" className="text-primary">→</span><span className="rounded-md border border-border px-3 py-2">CoIE</span><span aria-hidden="true" className="text-primary">→</span><span className="rounded-md border border-border px-3 py-2">TIE</span><span aria-hidden="true" className="text-primary">→</span><span className="rounded-md border border-border px-3 py-2">ChIE</span><span aria-hidden="true" className="text-primary">→</span><span className="rounded-md border border-border px-3 py-2">LLM Reasoning</span><span aria-hidden="true" className="text-primary">→</span><span className="rounded-md border border-border px-3 py-2">Evidence-grounded result</span></div>}
            {section.id === "troubleshooting" && <ul className="mt-4 list-disc space-y-2 pl-5 text-sm text-muted-foreground"><li>Check whether the repository is available and ready for analysis.</li><li>If the engine is unavailable, try again after its service is reachable.</li><li>Read partial-result limitations before drawing conclusions.</li></ul>}
            {section.id === "faq" && <div className="mt-4 grid gap-3"><details className="rounded-lg border border-border p-4"><summary className="cursor-pointer font-medium">Does a recommended test mean it was executed?</summary><p className="mt-2 text-sm text-muted-foreground">No. Test candidates indicate supported test relevance; they do not claim that tests ran.</p></details><details className="rounded-lg border border-border p-4"><summary className="cursor-pointer font-medium">Does missing evidence mean low risk?</summary><p className="mt-2 text-sm text-muted-foreground">No. Unknown or incomplete evidence remains explicit and does not become a low-risk result.</p></details></div>}
            </section>
          </TabsContent>)}
        </div>

        <aside className="grid content-start gap-4">
          <section className="rounded-xl border border-border bg-surface p-4">
            <h2 className="font-semibold">Quick links</h2>
            <div className="mt-3 grid gap-3 text-sm"><Link href="/" className="text-muted-foreground hover:text-primary">Start a new investigation ↗</Link><Link href="/repositories" className="text-muted-foreground hover:text-primary">Repositories ↗</Link><Link href="/settings" className="text-muted-foreground hover:text-primary">Settings ↗</Link></div>
          </section>
          <section className="rounded-xl border border-border bg-surface p-4">
            <h2 className="font-semibold">Still need help?</h2>
            <p className="mt-2 text-sm text-muted-foreground">Return to your workspace and open an investigation with the repository context you need.</p>
            <Link href="/" className="mt-4 inline-flex rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary-hover">Open CodeMind</Link>
          </section>
        </aside>
        </div>
      </Tabs>}
    </main>
  );
}
