import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ExecutionTimeline, type TimelineEvent } from "./execution-timeline";

function event(overrides: Partial<TimelineEvent> = {}): TimelineEvent {
  return {
    sequence: 1,
    status: "coie",
    message: "Analyzing repository",
    timestamp: "2026-10-21T10:00:00Z",
    engine: "coie",
    stage: "coie",
    stageState: "running",
    ...overrides,
  };
}

describe("ExecutionTimeline", () => {
  it("renders the complete supported investigation stage hierarchy", () => {
    render(<ExecutionTimeline events={[]} />);

    expect(screen.getByRole("heading", { name: /LLM \(Planning & Tool Invocation\)/i })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /CoIE \(Code Intelligence Engine\)/i })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /TIE \(Test Intelligence Engine\)/i })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /ChIE \(Change Intelligence Engine\)/i })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /LLM \(Reasoning & Generating Response\)/i })).toBeInTheDocument();
    expect(screen.getByText(/No operation events have been returned yet/i)).toBeInTheDocument();
  });

  it("orders events by sequence and deduplicates repeated progress event ids", () => {
    render(
      <ExecutionTimeline
        events={[
          event({ sequence: 4, progressEventId: "event-2", message: "Parsing source files" }),
          event({ sequence: 2, progressEventId: "event-1", message: "Cloning repository" }),
          event({ sequence: 5, progressEventId: "event-2", message: "Duplicate progress event" }),
        ]}
      />,
    );

    const coie = screen.getByText(/CoIE \(Code Intelligence Engine\)/i).closest("article");
    const labels = within(screen.getByLabelText("CoIE (Code Intelligence Engine) events"))
      .getAllByRole("listitem")
      .map((item) => item.textContent ?? "");

    expect(labels).toHaveLength(2);
    expect(labels[0]).toContain("Cloning repository");
    expect(labels[1]).toContain("Parsing source files");
    expect(coie).toHaveAttribute("data-stage-status", "running");
  });

  it("marks a previous stage complete when a later pipeline stage has started", () => {
    render(
      <ExecutionTimeline
        events={[
          event({
            sequence: 1,
            status: "planning",
            engine: undefined,
            stage: undefined,
            stageState: undefined,
            message: "Planning investigation",
          }),
          event({
            sequence: 2,
            status: "coie",
            engine: "coie",
            stage: "coie",
            stageState: "running",
            message: "Building code graph",
          }),
        ]}
      />,
    );

    const planning = screen.getByText(/LLM \(Planning & Tool Invocation\)/i).closest("article");
    expect(planning).toHaveAttribute("data-stage", "planning");
    expect(planning).toHaveAttribute("data-stage-status", "completed");
  });

  it("surfaces failed stages instead of treating them as completed", () => {
    render(
      <ExecutionTimeline
        events={[
          event({
            status: "failed",
            stageState: "failed",
            message: "The engine pipeline failed",
          }),
        ]}
      />,
    );

    const coie = screen.getByText(/CoIE \(Code Intelligence Engine\)/i).closest("article");
    expect(coie).toHaveAttribute("data-stage-status", "failed");
    expect(screen.getByText("Failed")).toBeInTheDocument();
  });

  it("keeps partial terminal results visibly distinct from success", () => {
    render(
      <ExecutionTimeline
        events={[
          event({
            sequence: 8,
            status: "partial",
            engine: undefined,
            stage: undefined,
            stageState: undefined,
            message: "Investigation completed with partial evidence",
          }),
        ]}
      />,
    );

    const finalStage = screen.getByText(/LLM \(Reasoning & Generating Response\)/i).closest("article");
    expect(finalStage).toHaveAttribute("data-stage-status", "partial");
    expect(within(finalStage!).getByText("Partial")).toBeInTheDocument();
  });

  it("uses the real event message and does not invent a progress percentage", () => {
    render(
      <ExecutionTimeline
        events={[
          event({
            message: "Building code graph in FalkorDB",
            stageState: "running",
          }),
        ]}
      />,
    );

    expect(screen.getByText("Building code graph in FalkorDB")).toBeInTheDocument();
    expect(screen.queryByText(/68%|67%|50%|100%/)).not.toBeInTheDocument();
  });
});
