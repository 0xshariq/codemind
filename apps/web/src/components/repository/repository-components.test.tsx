import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { RepositoryCard } from "./repository-card";
import { RepositoryList } from "./repository-list";

describe("repository components", () => {
  it("distinguishes indexed and indexing readiness", () => {
    render(<RepositoryCard repository={{ repositoryId: "repo", name: "nizaam-core", source: "github", readiness: "indexed", fileCount: 1200 }} />);
    expect(screen.getByText("Indexed")).toBeInTheDocument();
    expect(screen.getByText("1,200 files")).toBeInTheDocument();
  });
  it("renders an honest empty state when there are no matches", () => {
    render(<RepositoryList repositories={[]} />);
    expect(screen.getByText("No repositories yet")).toBeInTheDocument();
  });
});
