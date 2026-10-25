"use client";

import { useEffect, useState, type ReactNode } from "react";
import { Menu, X } from "lucide-react";
import { Sidebar } from "./sidebar";

export function AppShell({
  children,
  developmentSessionEnabled = false,
}: {
  children: ReactNode;
  developmentSessionEnabled?: boolean;
}) {
  const [mobileNavigationOpen, setMobileNavigationOpen] = useState(false);
  const [developmentSession, setDevelopmentSession] = useState<"connecting" | "ready" | "unavailable">(
    developmentSessionEnabled ? "connecting" : "ready",
  );

  useEffect(() => {
    if (!developmentSessionEnabled) return;
    const controller = new AbortController();

    fetch("/api/dev/session", { method: "POST", signal: controller.signal, cache: "no-store" })
      .then((response) => {
        if (!response.ok) throw new Error("Local session bootstrap failed");
        setDevelopmentSession("ready");
      })
      .catch((error: unknown) => {
        if (error instanceof DOMException && error.name === "AbortError") return;
        setDevelopmentSession("unavailable");
      });

    return () => controller.abort();
  }, [developmentSessionEnabled]);

  if (developmentSessionEnabled && developmentSession === "connecting") {
    return (
      <main className="grid min-h-svh place-items-center bg-background p-6" aria-busy="true">
        <p role="status" className="text-sm text-muted-foreground">
          Connecting to the local CodeMind workspace…
        </p>
      </main>
    );
  }

  return (
    <div className="flex min-h-svh bg-background">
      <div className="hidden w-[320px] shrink-0 border-r border-sidebar-border md:block">
        <Sidebar />
      </div>

      <div className="flex min-w-0 flex-1 flex-col">
        {developmentSession === "unavailable" && (
          <p role="alert" className="border-b border-danger/40 bg-danger/10 px-4 py-2 text-sm text-danger">
            The local workspace session could not be started. Check the development authentication settings.
          </p>
        )}

        <header className="sticky top-0 z-30 flex min-h-14 items-center border-b border-border bg-background px-4 md:hidden">
          <button
            type="button"
            aria-label={mobileNavigationOpen ? "Close navigation" : "Open navigation"}
            aria-expanded={mobileNavigationOpen}
            aria-controls="mobile-primary-navigation"
            onClick={() => setMobileNavigationOpen((open) => !open)}
            className="inline-grid size-9 place-items-center rounded-md text-muted-foreground transition-colors hover:bg-surface-alt hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
          >
            {mobileNavigationOpen ? <X aria-hidden="true" className="size-5" /> : <Menu aria-hidden="true" className="size-5" />}
          </button>
          <span className="ml-3 text-base font-semibold tracking-tight">
            Code<span className="text-primary">Mind</span>
          </span>
        </header>

        {mobileNavigationOpen && (
          <div id="mobile-primary-navigation" className="fixed inset-0 top-14 z-40 md:hidden">
            <button
              type="button"
              aria-label="Close navigation"
              className="absolute inset-0 bg-black/60"
              onClick={() => setMobileNavigationOpen(false)}
            />
            <div className="relative h-full w-[min(88vw,280px)] shadow-lg">
              <Sidebar onNavigate={() => setMobileNavigationOpen(false)} />
            </div>
          </div>
        )}

        <div className="min-w-0 flex-1">{children}</div>
      </div>
    </div>
  );
}
