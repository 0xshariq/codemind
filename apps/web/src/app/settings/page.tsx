"use client";

import { Suspense, useEffect, useMemo, useState } from "react";
import { useSearchParams } from "next/navigation";
import type { ReactNode } from "react";
import { Bell, BookOpen, BrainCircuit, Keyboard, LockKeyhole, Palette, Settings2, ShieldCheck } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent } from "@/components/ui/tabs";
import { SettingsNavigation } from "@/components/settings/settings-navigation";

type Repository = { repositoryId: string; name: string };
type SettingsRepositoryResponse = { repositories?: unknown[] };

const sections = [
  { id: "general", title: "General", description: "Workspace defaults", icon: Settings2 },
  { id: "appearance", title: "Appearance", description: "Theme and display", icon: Palette },
  { id: "ai-models", title: "AI & Models", description: "Provider configuration", icon: BrainCircuit },
  { id: "repository-analysis", title: "Repository Analysis", description: "Analysis behavior", icon: BookOpen },
  { id: "engines", title: "CodeMind Engines", description: "CoIE, TIE and ChIE", icon: BrainCircuit },
  { id: "notifications", title: "Notifications", description: "Alerts and updates", icon: Bell },
  { id: "privacy", title: "Privacy & Data", description: "Data handling", icon: ShieldCheck },
  { id: "shortcuts", title: "Keyboard Shortcuts", description: "Keyboard access", icon: Keyboard },
  { id: "security", title: "Security", description: "Access and sessions", icon: LockKeyhole },
];

function parseRepository(value: unknown): Repository | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  return typeof record.repositoryId === "string" && typeof record.name === "string"
    ? { repositoryId: record.repositoryId, name: record.name }
    : null;
}

function SettingsSection({ id, title, description, children }: { id: string; title: string; description: string; children: ReactNode }) {
  return <section id={id} className="scroll-mt-6"><Card><CardHeader><h2 className="font-heading text-base font-medium">{title}</h2><CardDescription>{description}</CardDescription></CardHeader><CardContent className="grid gap-4">{children}</CardContent></Card></section>;
}

function PreferenceRow({ title, description, children }: { title: string; description: string; children: ReactNode }) {
  return <div className="grid gap-3 border-b border-border pb-4 last:border-0 last:pb-0 sm:grid-cols-[minmax(0,1fr)_minmax(180px,280px)] sm:items-center"><div><h3 className="text-sm font-medium">{title}</h3><p className="mt-1 text-sm text-muted-foreground">{description}</p></div>{children}</div>;
}

export default function SettingsPage() {
  return (
    <Suspense fallback={<main className="mx-auto grid w-full max-w-7xl gap-6 p-4 sm:p-6 lg:p-8" aria-busy="true"><p role="status" className="text-sm text-muted-foreground">Loading settings…</p></main>}>
      <SettingsPageContent />
    </Suspense>
  );
}

function SettingsPageContent() {
  const searchParams = useSearchParams();
  const query = (searchParams.get("q") ?? "").trim().toLowerCase();
  const [repositories, setRepositories] = useState<Repository[]>([]);
  const [repositoriesAvailable, setRepositoriesAvailable] = useState(false);
  const [defaultRepositoryId, setDefaultRepositoryId] = useState("");
  const [activeSection, setActiveSection] = useState(searchParams.get("section") ?? sections[0].id);

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
  const selectedSection = visibleSections.some(({ id }) => id === activeSection) ? activeSection : visibleSections[0]?.id ?? "";
  function saveDefaultRepository(value: string) {
    setDefaultRepositoryId(value);
    if (value) window.localStorage.setItem("codemind.defaultRepositoryId", value);
    else window.localStorage.removeItem("codemind.defaultRepositoryId");
  }

  return (
    <main className="mx-auto grid w-full max-w-7xl gap-6 p-4 sm:p-6 lg:p-8">
      <header className="flex flex-col justify-between gap-4 sm:flex-row sm:items-end">
        <div><p className="text-sm text-muted-foreground">WORKSPACE</p><h1 className="mt-1 text-3xl font-semibold tracking-tight">Settings</h1><p className="mt-2 text-sm text-muted-foreground">Review workspace preferences and server-managed analysis configuration.</p></div>
        <form action="/settings" role="search" className="flex min-h-10 items-center gap-2 rounded-lg border border-border bg-surface px-3 focus-within:border-primary"><label htmlFor="settings-search" className="sr-only">Search settings</label><input id="settings-search" name="q" type="search" defaultValue={query} placeholder="Search settings" className="w-full bg-transparent text-sm outline-none placeholder:text-muted-foreground sm:w-64" /><button type="submit" className="text-xs text-primary">Search</button></form>
      </header>

      <Tabs value={selectedSection} onValueChange={(value) => { if (value) setActiveSection(value); }} orientation="vertical">
        <div className="grid gap-5 lg:grid-cols-[240px_minmax(0,1fr)]">
          {visibleSections.length > 0 && <SettingsNavigation items={visibleSections} />}

          {visibleSections.length === 0 ? <Card><CardHeader><CardTitle>No matching settings</CardTitle><CardDescription>Try a broader search term.</CardDescription></CardHeader></Card> : <TabsContent value={selectedSection} className="grid content-start gap-4">

          {selectedSection === "general" && <SettingsSection id="general" title="General" description="Workspace defaults and behavior.">
            <PreferenceRow title="Default repository" description="Choose the repository preselected when starting a new investigation.">
              <select aria-label="Default repository" disabled={!repositoriesAvailable} value={defaultRepositoryId} onChange={(event) => saveDefaultRepository(event.target.value)} className="min-h-10 rounded-md border border-border bg-surface px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-60">
                <option value="">No default</option>{repositories.map((repository) => <option key={repository.repositoryId} value={repository.repositoryId}>{repository.name}</option>)}
              </select>
            </PreferenceRow>
            {!repositoriesAvailable && <p role="status" className="text-xs text-warning">Repository choices are unavailable.</p>}
            <PreferenceRow title="Analysis policy" description="The Agent Runtime and Rust engine apply the approved investigation limits."><Badge variant="outline">Server managed</Badge></PreferenceRow>
          </SettingsSection>}

          {selectedSection === "appearance" && <SettingsSection id="appearance" title="Appearance" description="CodeMind uses the supplied dark-first design system.">
            <PreferenceRow title="Theme" description="The current product interface follows the CodeMind dark theme."><select aria-label="Theme" disabled value="dark" className="min-h-10 rounded-md border border-border bg-surface px-3 text-sm disabled:opacity-70"><option value="dark">Dark (current design)</option></select></PreferenceRow>
            <PreferenceRow title="UI density" description="Density preferences are not connected to a persisted display setting."><select aria-label="UI density" disabled value="unavailable" className="min-h-10 rounded-md border border-border bg-surface px-3 text-sm disabled:opacity-70"><option value="unavailable">Unavailable</option></select></PreferenceRow>
            <PreferenceRow title="Graph appearance" description="Graph rendering preferences are unavailable until a graph view is connected."><select aria-label="Graph appearance" disabled value="unavailable" className="min-h-10 rounded-md border border-border bg-surface px-3 text-sm disabled:opacity-70"><option value="unavailable">Unavailable</option></select></PreferenceRow>
            <PreferenceRow title="Technical typography" description="Technical identifiers use a JetBrains Mono / Fira Code-compatible system font stack."><Badge variant="outline">System code font</Badge></PreferenceRow>
          </SettingsSection>}

          {selectedSection === "ai-models" && <SettingsSection id="ai-models" title="AI & Models" description="Provider credentials and model execution stay on the server.">
            <PreferenceRow title="Preferred model" description="The server selects CODEMIND_LLM_MODEL. This workspace does not expose model overrides."><select aria-label="Preferred model" disabled value="server-managed" className="min-h-10 rounded-md border border-border bg-surface px-3 text-sm disabled:opacity-70"><option value="server-managed">Server managed</option></select></PreferenceRow>
            <PreferenceRow title="Provider" description="Provider credentials are configured by the server administrator."><select aria-label="Provider" disabled value="server-managed" className="min-h-10 rounded-md border border-border bg-surface px-3 text-sm disabled:opacity-70"><option value="server-managed">Server managed</option></select></PreferenceRow>
            <PreferenceRow title="Reasoning mode" description="Reasoning depth is controlled by the server runtime."><select aria-label="Reasoning mode" disabled value="unavailable" className="min-h-10 rounded-md border border-border bg-surface px-3 text-sm disabled:opacity-70"><option value="unavailable">Unavailable</option></select></PreferenceRow>
            <PreferenceRow title="Response style" description="Response formatting is currently controlled by the server prompt."><select aria-label="Response style" disabled value="server-managed" className="min-h-10 rounded-md border border-border bg-surface px-3 text-sm disabled:opacity-70"><option value="server-managed">Server managed</option></select></PreferenceRow>
            <PreferenceRow title="Context window and temperature" description="These model parameters are not exposed by the current provider contract."><Badge variant="outline">Unavailable</Badge></PreferenceRow>
            <PreferenceRow title="Citations and structured output" description="The current UI renders canonical evidence separately; these model toggles are not configurable."><Badge variant="outline">Server managed</Badge></PreferenceRow>
            <p className="text-xs text-muted-foreground">Credentials are never returned to the browser or stored as a client preference.</p>
          </SettingsSection>}

          {selectedSection === "repository-analysis" && <SettingsSection id="repository-analysis" title="Repository Analysis" description="Analysis uses the canonical repository snapshot and engine policy.">
            <PreferenceRow title="Snapshot and graph scope" description="These are bound to the selected repository context for each operation."><Badge variant="outline">Context bound</Badge></PreferenceRow>
            <PreferenceRow title="Traversal and result limits" description="Limits are enforced by the server and Rust engine, not browser settings."><Badge variant="outline">Server managed</Badge></PreferenceRow>
          </SettingsSection>}

          {selectedSection === "engines" && <SettingsSection id="engines" title="CodeMind Engines" description="The Rust engine owns canonical code, test and change intelligence.">
            <div className="grid gap-3 sm:grid-cols-3">{[
              ["CoIE", "Code Intelligence Engine", "Code structure and relationships."],
              ["TIE", "Test Intelligence Engine", "Test declarations and supported mappings."],
              ["ChIE", "Change Intelligence Engine", "Impact, qualitative risk and evidence paths."],
            ].map(([name, title, description]) => <div key={name} className="rounded-lg border border-border bg-surface p-4"><p className="font-mono font-semibold text-accent">{name}</p><p className="mt-1 text-sm font-medium">{title}</p><p className="mt-2 text-xs text-muted-foreground">{description}</p></div>)}</div>
          </SettingsSection>}

          {selectedSection === "notifications" && <SettingsSection id="notifications" title="Notifications" description="Notification delivery preferences are not available in this workspace yet."><PreferenceRow title="Analysis completed or failed" description="A notification delivery service is not configured."><input aria-label="Analysis notifications" type="checkbox" disabled className="size-4 accent-primary" /></PreferenceRow><PreferenceRow title="Repository indexing and long-running analysis" description="A notification delivery service is not configured."><input aria-label="Repository notifications" type="checkbox" disabled className="size-4 accent-primary" /></PreferenceRow></SettingsSection>}

          {selectedSection === "privacy" && <SettingsSection id="privacy" title="Privacy & Data" description="Storage and retention follow the configured backend services.">
            <PreferenceRow title="Conversation and analysis storage" description="Persisted state is scoped by the authenticated workspace and repository context."><Badge variant="outline">Workspace scoped</Badge></PreferenceRow>
            <PreferenceRow title="Data retention" description="Retention policy is managed by the configured backend."><select aria-label="Data retention" disabled value="server-managed" className="min-h-10 rounded-md border border-border bg-surface px-3 text-sm disabled:opacity-70"><option value="server-managed">Server managed</option></select></PreferenceRow>
            <PreferenceRow title="Clear stored data" description="The current service contract does not expose a safe deletion operation."><button type="button" disabled className="min-h-9 rounded-md border border-danger/50 px-3 text-sm text-danger disabled:opacity-50">Unavailable</button></PreferenceRow>
            <p className="text-sm text-muted-foreground">Data deletion and retention controls are not exposed by the current service contract.</p>
          </SettingsSection>}

          {selectedSection === "shortcuts" && <SettingsSection id="shortcuts" title="Keyboard Shortcuts" description="Standard browser and accessible control navigation is supported."><div className="grid gap-2 text-sm sm:grid-cols-2"><p><kbd className="rounded border border-border bg-surface-alt px-2 py-1 font-mono text-xs">Tab</kbd><span className="ml-3 text-muted-foreground">Move to the next control</span></p><p><kbd className="rounded border border-border bg-surface-alt px-2 py-1 font-mono text-xs">Shift + Tab</kbd><span className="ml-3 text-muted-foreground">Move to the previous control</span></p><p><kbd className="rounded border border-border bg-surface-alt px-2 py-1 font-mono text-xs">Enter</kbd><span className="ml-3 text-muted-foreground">Activate a focused control</span></p></div></SettingsSection>}

          {selectedSection === "security" && <SettingsSection id="security" title="Security" description="Session and scope enforcement are handled by the application server and Rust boundary."><div className="flex items-start gap-3"><LockKeyhole aria-hidden="true" className="mt-0.5 size-5 text-success" /><p className="text-sm text-muted-foreground">Protected API requests require a signed session. Repository and snapshot scope is validated before canonical data is returned.</p></div></SettingsSection>}
          </TabsContent>}
        </div>
      </Tabs>
    </main>
  );
}
