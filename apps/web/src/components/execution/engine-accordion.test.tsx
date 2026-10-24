import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { EngineAccordion } from "./engine-accordion";

describe("EngineAccordion", () => {
  it("keeps CoIE, TIE and ChIE as separate engine stages", () => {
    render(<EngineAccordion events={[
      { sequence: 1, engine: "CoIE", stage: "parse", stageState: "completed", message: "Parsing source files", timestamp: "2026-10-21T10:00:00Z" },
      { sequence: 2, engine: "TIE", stage: "discover", stageState: "running", message: "Discovering test files", timestamp: "2026-10-21T10:00:01Z" },
      { sequence: 3, engine: "ChIE", stage: "impact", stageState: "pending", message: "Waiting", timestamp: "2026-10-21T10:00:02Z" },
    ]} />);
    expect(screen.getByText(/CoIE/)).toBeInTheDocument();
    expect(screen.getByText(/TIE/)).toBeInTheDocument();
    expect(screen.getByText(/ChIE/)).toBeInTheDocument();
    expect(screen.getByText("Discovering test files")).toBeInTheDocument();
  });

  it("shows an honest no-events state", () => {
    render(<EngineAccordion events={[]} />);
    expect(screen.getAllByText(/No Rust progress event/)).toHaveLength(3);
  });
});
