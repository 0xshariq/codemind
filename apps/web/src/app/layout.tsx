import type { Metadata } from "next";
import { Inter } from "next/font/google";
import type { ReactNode } from "react";
import { AppShell } from "@/components/shell/app-shell";
import "./globals.css";

const inter = Inter({ subsets: ["latin"], variable: "--font-ui" });

export const metadata: Metadata = {
  title: {
    default: "CodeMind — AI codebase investigation",
    template: "%s | CodeMind",
  },
  description: "Investigate code relationships, change impact, test intelligence and evidence with CodeMind.",
};

export default function RootLayout({ children }: { children: ReactNode }) {
  const developmentSessionEnabled = process.env.NODE_ENV !== "production"
    && process.env.CODEMIND_DEV_AUTH_ENABLED === "true";

  return (
    <html lang="en" className={`${inter.variable} ${inter.className} min-h-full antialiased`}>
      <body className="min-h-full">
        <AppShell developmentSessionEnabled={developmentSessionEnabled}>{children}</AppShell>
      </body>
    </html>
  );
}
