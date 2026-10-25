"use client";

import { Suspense, useEffect, useMemo, useState, type ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import { useSearchParams } from "next/navigation";
import { Bell, BookOpen, BrainCircuit, Check, Info, Keyboard, Palette, Settings2, ShieldCheck } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { SettingsNavigation } from "@/components/settings/settings-navigation";

type Repository = { repositoryId: string; name: string };
type SettingsRepositoryResponse = { repositories?: unknown[] };

type ToggleProps = { label: string; value: boolean; onChange: (value: boolean) => void; disabled?: boolean };

const sections = [
  { id: "general", title: "General", description: "Basic preferences and behavior", icon: Settings2 },
  { id: "appearance", title: "Appearance", description: "Theme, layout and display", icon: Palette },
  { id: "ai-models", title: "AI & Models", description: "Model configuration and behavior", icon: BrainCircuit },
  { id: "repository-analysis", title: "Repository Analysis", description: "Analysis depth and preferences", icon: BookOpen },
  { id: "engines", title: "CodeMind Engines", description: "CoIE, TIE and ChIE configuration", icon: BrainCircuit },
  { id: "notifications", title: "Notifications", description: "Alerts and updates", icon: Bell },
  { id: "privacy", title: "Privacy & Data", description: "Data storage and retention", icon: ShieldCheck },
  { id: "shortcuts", title: "Keyboard Shortcuts", description: "Productivity shortcuts", icon: Keyboard },
  { id: "about", title: "About", description: "Version, links and open source", icon: Info },
];

function parseRepository(value: unknown): Repository | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  return typeof record.repositoryId === "string" && typeof record.name === "string" ? { repositoryId: record.repositoryId, name: record.name } : null;
}

function readPreference<T>(key: string, fallback: T): T {
  try {
    const raw = window.localStorage.getItem(`codemind.ui.${key}`);
    return raw === null ? fallback : JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

function usePreference<T>(key: string, fallback: T) {
  const [value, setValue] = useState<T>(fallback);
  useEffect(() => setValue(readPreference(key, fallback)), [key, fallback]);
  function update(next: T) {
    setValue(next);
    try { window.localStorage.setItem(`codemind.ui.${key}`, JSON.stringify(next)); } catch { /* local preferences are optional */ }
  }
  return [value, update] as const;
}

function SettingsSection({ id, title, description, icon, children }: { id: string; title: string; description: string; icon?: LucideIcon; children: ReactNode }) {
  const Icon = icon ?? Settings2;
  return <section id={id} className="scroll-mt-6"><Card className="overflow-hidden"><CardHeader className="border-b border-border/60"><div className="flex items-start gap-3"><span className="grid size-10 shrink-0 place-items-center rounded-lg bg-accent/10 text-accent"><Icon aria-hidden="true" className="size-5" /></span><div><CardTitle>{title}</CardTitle><CardDescription className="mt-1">{description}</CardDescription></div></div></CardHeader><CardContent className="grid gap-4 p-4 sm:p-5">{children}</CardContent></Card></section>;
}

function PreferenceRow({ title, description, children }: { title: string; description: string; children: ReactNode }) {
  return <div className="grid gap-3 border-b border-border/60 pb-4 last:border-0 last:pb-0 sm:grid-cols-[minmax(0,1fr)_minmax(180px,1.1fr)] sm:items-center"><div><h3 className="text-sm font-medium">{title}</h3><p className="mt-1 text-xs leading-5 text-muted-foreground">{description}</p></div><div className="sm:justify-self-end sm:w-full">{children}</div></div>;
}

function Toggle({ label, value, onChange, disabled = false }: ToggleProps) {
  return <button type="button" role="switch" aria-label={label} aria-checked={value} disabled={disabled} onClick={() => onChange(!value)} className={`relative inline-flex h-6 w-11 shrink-0 items-center rounded-full border transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 ${value ? "border-primary bg-primary" : "border-border bg-surface-alt"}`}><span className={`size-4 rounded-full bg-white shadow-sm transition-transform ${value ? "translate-x-6" : "translate-x-1"}`} /></button>;
}

function SelectControl({ label, value, onChange, options }: { label: string; value: string; onChange: (value: string) => void; options: string[] }) {
  return <select aria-label={label} value={value} onChange={(event) => onChange(event.target.value)} className="min-h-10 w-full rounded-md border border-border bg-background px-3 text-sm outline-none hover:border-primary/60 focus-visible:ring-2 focus-visible:ring-ring">{options.map((option) => <option key={option} value={option}>{option}</option>)}</select>;
}

export default function SettingsPage() {
  return <Suspense fallback={<main className="mx-auto grid w-full max-w-7xl p-4 sm:p-6 lg:p-8" aria-busy="true"><p role="status" className="text-sm text-muted-foreground">Loading settings…</p></main>}><SettingsPageContent /></Suspense>;
}

function SettingsPageContent() {
  const searchParams = useSearchParams();
  const query = (searchParams.get("q") ?? "").trim().toLowerCase();
  const [repositories, setRepositories] = useState<Repository[]>([]);
  const [repositoriesAvailable, setRepositoriesAvailable] = useState(false);
  const [defaultRepositoryId, setDefaultRepositoryId] = useState("");
  const [activeSection, setActiveSection] = useState(searchParams.get("section") ?? "ai-models");

  const [model, setModel] = usePreference("model", "Auto (Recommended)");
  const [reasoning, setReasoning] = usePreference("reasoning", "Balanced (Recommended)");
  const [responseStyle, setResponseStyle] = usePreference("responseStyle", "Detailed");
  const [contextWindow, setContextWindow] = usePreference("contextWindow", "Auto (Recommended)");
  const [temperature, setTemperature] = usePreference("temperature", 0.2);
  const [citations, setCitations] = usePreference("citations", true);
  const [structuredOutput, setStructuredOutput] = usePreference("structuredOutput", true);
  const [testAnalysis, setTestAnalysis] = usePreference("testAnalysis", true);
  const [historyAnalysis, setHistoryAnalysis] = usePreference("historyAnalysis", true);
  const [autoReanalyze, setAutoReanalyze] = usePreference("autoReanalyze", false);
  const [analysisCompleted, setAnalysisCompleted] = usePreference("analysisCompleted", true);
  const [analysisFailed, setAnalysisFailed] = usePreference("analysisFailed", true);
  const [repositoryIndexing, setRepositoryIndexing] = usePreference("repositoryIndexing", true);
  const [longRunning, setLongRunning] = usePreference("longRunning", true);
  const [conversationHistory, setConversationHistory] = usePreference("conversationHistory", true);
  const [analysisHistory, setAnalysisHistory] = usePreference("analysisHistory", true);
  const [retention, setRetention] = usePreference("retention", "30 days");

  useEffect(() => {
    setDefaultRepositoryId(window.localStorage.getItem("codemind.defaultRepositoryId") ?? "");
    const controller = new AbortController();
    fetch("/api/repositories", { signal: controller.signal, cache: "no-store" })
      .then(async (response) => {
        if (!response.ok) throw new Error("unavailable");
        const value = await response.json() as SettingsRepositoryResponse;
        setRepositories((value.repositories ?? []).map(parseRepository).filter((item): item is Repository => item !== null));
        setRepositoriesAvailable(true);
      })
      .catch((error: unknown) => {
        if (error instanceof DOMException && error.name === "AbortError") return;
        setRepositoriesAvailable(false);
      });
    return () => controller.abort();
  }, []);

  const visibleSections = useMemo(() => sections.filter((section) => !query || `${section.title} ${section.description}`.toLowerCase().includes(query)), [query]);

  function saveDefaultRepository(value: string) {
    setDefaultRepositoryId(value);
    if (value) window.localStorage.setItem("codemind.defaultRepositoryId", value); else window.localStorage.removeItem("codemind.defaultRepositoryId");
  }

  function selectSection(id: string) {
    setActiveSection(id);
    document.getElementById(id)?.scrollIntoView({ behavior: "smooth", block: "start" });
  }

  return (
    <main className="mx-auto grid w-full max-w-[1500px] gap-6 p-4 sm:p-6 lg:p-8">
      <header className="flex flex-col justify-between gap-4 sm:flex-row sm:items-end"><div><p className="text-xs uppercase tracking-[0.16em] text-muted-foreground">Workspace</p><h1 className="mt-1 text-3xl font-semibold tracking-tight">Settings</h1><p className="mt-2 text-sm text-muted-foreground">Customize CodeMind to match your workflow and preferences.</p></div><form action="/settings" role="search" className="flex min-h-10 items-center gap-2 rounded-lg border border-border bg-surface px-3 focus-within:border-primary"><label htmlFor="settings-search" className="sr-only">Search settings</label><input id="settings-search" name="q" type="search" defaultValue={query} placeholder="Search settings" className="w-full bg-transparent text-sm outline-none placeholder:text-muted-foreground sm:w-64" /><button type="submit" className="text-xs text-primary">Search</button></form></header>

      <div className="grid gap-5 lg:grid-cols-[240px_minmax(0,1fr)]">
        <SettingsNavigation items={visibleSections} activeId={activeSection} onSelect={selectSection} />
        <div className="grid content-start gap-4">
          {visibleSections.length === 0 && <Card><CardHeader><CardTitle>No matching settings</CardTitle><CardDescription>Try a broader search term.</CardDescription></CardHeader></Card>}

          {visibleSections.some((section) => section.id === "general") && <SettingsSection id="general" title="General" description="Basic workspace preferences and behavior." icon={Settings2}>
            <PreferenceRow title="Default repository" description="Choose the repository preselected when starting a new investigation."><select aria-label="Default repository" disabled={!repositoriesAvailable} value={defaultRepositoryId} onChange={(event) => saveDefaultRepository(event.target.value)} className="min-h-10 w-full rounded-md border border-border bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-60"><option value="">No default</option>{repositories.map((repository) => <option key={repository.repositoryId} value={repository.repositoryId}>{repository.name}</option>)}</select></PreferenceRow>
            {!repositoriesAvailable && <p role="status" className="text-xs text-warning">Repository choices are unavailable.</p>}
          </SettingsSection>}

          {visibleSections.some((section) => section.id === "appearance") && <SettingsSection id="appearance" title="Appearance" description="Theme, layout and display preferences." icon={Palette}>
            <PreferenceRow title="Theme" description="CodeMind currently uses the dark-first product design."><Badge variant="outline">Dark</Badge></PreferenceRow>
            <PreferenceRow title="UI density" description="Controls the amount of information shown in dense analytical screens."><SelectControl label="UI density" value="Comfortable" onChange={() => undefined} options={["Comfortable", "Compact"]} /></PreferenceRow>
            <PreferenceRow title="Graph appearance" description="The graph view uses the product graph tokens and accessible relationship labels."><SelectControl label="Graph appearance" value="Standard" onChange={() => undefined} options={["Standard", "High contrast"]} /></PreferenceRow>
          </SettingsSection>}

          {visibleSections.some((section) => section.id === "ai-models") && <SettingsSection id="ai-models" title="AI & Models" description="Configure AI models and response behavior.">
            <div className="grid gap-4 lg:grid-cols-2">
              <div className="grid gap-4"><PreferenceRow title="Preferred Model" description="Select the model to use for analysis and reasoning."><SelectControl label="Preferred Model" value={model} onChange={setModel} options={["Auto (Recommended)", "Server default", "Fast model"]} /></PreferenceRow><PreferenceRow title="Reasoning Mode" description="Control the depth of analysis and reasoning."><SelectControl label="Reasoning Mode" value={reasoning} onChange={setReasoning} options={["Balanced (Recommended)", "Fast", "Deep"]} /></PreferenceRow><PreferenceRow title="Response Style" description="Choose how detailed responses should be."><SelectControl label="Response Style" value={responseStyle} onChange={setResponseStyle} options={["Detailed", "Concise", "Technical"]} /></PreferenceRow></div>
              <div className="grid gap-4"><PreferenceRow title="Context Window" description="Maximum context size used for code analysis."><SelectControl label="Context Window" value={contextWindow} onChange={setContextWindow} options={["Auto (Recommended)", "Standard", "Large"]} /></PreferenceRow><PreferenceRow title="Temperature" description="Lower values keep responses more focused."><div className="flex items-center gap-3"><input aria-label="Temperature" type="range" min="0" max="1" step="0.1" value={temperature} onChange={(event) => setTemperature(Number(event.target.value))} className="w-full accent-primary" /><span className="min-w-12 rounded-md border border-border px-2 py-1 text-center text-xs">{Number(temperature).toFixed(1)}</span></div></PreferenceRow><PreferenceRow title="Citations" description="Include source and evidence references in responses."><Toggle label="Citations" value={citations} onChange={setCitations} /></PreferenceRow><PreferenceRow title="Structured Output" description="Format responses with clear sections and structure."><Toggle label="Structured Output" value={structuredOutput} onChange={setStructuredOutput} /></PreferenceRow></div>
            </div>
          </SettingsSection>}

          {visibleSections.some((section) => section.id === "repository-analysis") && <SettingsSection id="repository-analysis" title="Repository Analysis" description="Configure how repositories are analyzed.">
            <div className="grid gap-4 lg:grid-cols-2"><div className="grid gap-4"><PreferenceRow title="Default Analysis Depth" description="Controls how deeply the codebase is analyzed."><SelectControl label="Default Analysis Depth" value="Standard (Recommended)" onChange={() => undefined} options={["Standard (Recommended)", "Deep", "Fast"]} /></PreferenceRow><PreferenceRow title="Maximum Traversal Depth" description="Maximum depth for dependency traversal."><SelectControl label="Maximum Traversal Depth" value="10 levels" onChange={() => undefined} options={["5 levels", "10 levels", "20 levels"]} /></PreferenceRow><PreferenceRow title="Include Patterns" description="File patterns included in analysis."><input aria-label="Include Patterns" defaultValue="src/**, lib/**, app/**" className="min-h-10 w-full rounded-md border border-border bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring" /></PreferenceRow></div><div className="grid gap-4"><PreferenceRow title="Test Analysis" description="Analyze tests and test dependencies."><Toggle label="Test Analysis" value={testAnalysis} onChange={setTestAnalysis} /></PreferenceRow><PreferenceRow title="History Analysis" description="Include supported repository history in analysis."><Toggle label="History Analysis" value={historyAnalysis} onChange={setHistoryAnalysis} /></PreferenceRow><PreferenceRow title="Auto Re-analyze" description="Automatically re-analyze when repository changes are detected."><Toggle label="Auto Re-analyze" value={autoReanalyze} onChange={setAutoReanalyze} /></PreferenceRow><PreferenceRow title="Large File Handling" description="Choose how very large files are treated."><SelectControl label="Large File Handling" value="Skip files > 1MB" onChange={() => undefined} options={["Skip files > 1MB", "Include all files", "Ask before including"]} /></PreferenceRow></div></div>
          </SettingsSection>}

          {visibleSections.some((section) => section.id === "engines") && <SettingsSection id="engines" title="CodeMind Engines" description="Configure the analysis engines used by CodeMind."><div className="grid gap-3 md:grid-cols-3">{[["CoIE", "Code Intelligence Engine", "Analyzes code structure, dependencies and relationships."], ["TIE", "Test Intelligence Engine", "Analyzes tests, coverage and test impact."], ["ChIE", "Change Intelligence Engine", "Analyzes potential changes, impact and risk."]].map(([name, title, description]) => <div key={name} className="rounded-lg border border-border bg-background/40 p-4"><div className="flex items-center justify-between gap-2"><div><p className="font-mono font-semibold text-accent">{name}</p><p className="mt-1 text-sm font-medium">{title}</p></div><Badge variant="default"><Check className="size-3" />Enabled</Badge></div><p className="mt-3 text-xs leading-5 text-muted-foreground">{description}</p></div>)}</div></SettingsSection>}

          {visibleSections.some((section) => section.id === "notifications") && <SettingsSection id="notifications" title="Notifications" description="Configure when and how you receive notifications."><div className="grid gap-4 lg:grid-cols-2"><div className="grid gap-4"><PreferenceRow title="Analysis Completed" description="Get notified when an analysis is finished."><Toggle label="Analysis Completed" value={analysisCompleted} onChange={setAnalysisCompleted} /></PreferenceRow><PreferenceRow title="Analysis Failed" description="Get notified when an analysis fails."><Toggle label="Analysis Failed" value={analysisFailed} onChange={setAnalysisFailed} /></PreferenceRow></div><div className="grid gap-4"><PreferenceRow title="Repository Indexing" description="Get notified when repository indexing is complete."><Toggle label="Repository Indexing" value={repositoryIndexing} onChange={setRepositoryIndexing} /></PreferenceRow><PreferenceRow title="Long Running Analysis" description="Get notified for analyses taking longer than five minutes."><Toggle label="Long Running Analysis" value={longRunning} onChange={setLongRunning} /></PreferenceRow></div></div></SettingsSection>}

          {visibleSections.some((section) => section.id === "privacy") && <SettingsSection id="privacy" title="Privacy & Data" description="Manage your data, privacy and retention settings."><PreferenceRow title="Conversation History" description="Store conversation history locally for this workspace."><Toggle label="Conversation History" value={conversationHistory} onChange={setConversationHistory} /></PreferenceRow><PreferenceRow title="Analysis History" description="Store analysis results and evidence for later review."><Toggle label="Analysis History" value={analysisHistory} onChange={setAnalysisHistory} /></PreferenceRow><PreferenceRow title="Data Retention" description="How long to keep conversation and analysis data."><SelectControl label="Data Retention" value={retention} onChange={setRetention} options={["7 days", "30 days", "90 days", "Forever"]} /></PreferenceRow><PreferenceRow title="Clear All Data" description="The current service contract does not expose a safe server-side deletion operation."><button type="button" disabled className="min-h-10 w-full rounded-md border border-danger/50 px-3 text-sm text-danger disabled:cursor-not-allowed disabled:opacity-50">Unavailable</button></PreferenceRow></SettingsSection>}

          {visibleSections.some((section) => section.id === "shortcuts") && <SettingsSection id="shortcuts" title="Keyboard Shortcuts" description="Customize keyboard shortcuts for faster navigation." icon={Keyboard}><div className="grid gap-3 sm:grid-cols-2"><Shortcut keys="Ctrl K" label="Search Conversations" /><Shortcut keys="Ctrl P" label="Open Command Palette" /><Shortcut keys="Enter" label="Send Message" /><Shortcut keys="Shift Enter" label="New Line" /><Shortcut keys="/" label="Focus Input" /></div></SettingsSection>}

          {visibleSections.some((section) => section.id === "about") && <SettingsSection id="about" title="About" description="Version information, links and open source details." icon={Info}><div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between"><div className="flex items-center gap-3"><span className="grid size-10 place-items-center rounded-lg bg-accent/10 text-accent"><BrainCircuit className="size-5" /></span><div><p className="font-semibold">CodeMind</p><p className="text-xs text-muted-foreground">Version 1.0.0 · AI codebase investigation agent</p></div></div><div className="flex flex-wrap gap-2"><a href="/help" className="inline-flex min-h-9 items-center rounded-md border border-border px-3 text-xs hover:bg-surface-alt">Documentation</a><a href="https://github.com" target="_blank" rel="noreferrer" className="inline-flex min-h-9 items-center rounded-md border border-border px-3 text-xs hover:bg-surface-alt">GitHub</a></div></div></SettingsSection>}
        </div>
      </div>
    </main>
  );
}

function Shortcut({ keys, label }: { keys: string; label: string }) { return <div className="flex items-center justify-between gap-3 rounded-lg border border-border bg-background/40 px-3 py-2.5"><span className="text-sm">{label}</span><kbd className="rounded border border-border bg-surface px-2 py-1 font-mono text-xs">{keys}</kbd></div>; }
