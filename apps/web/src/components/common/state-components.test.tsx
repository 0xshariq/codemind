import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CompletenessNotice } from "./completeness-notice";
import { EmptyState } from "./empty-state";
import { ErrorState } from "./error-state";
import { LoadingState } from "./loading-state";

describe("shared state components", () => {
  it("keeps partial and truncated results explicit", () => {
    render(<CompletenessNotice completeness="truncated" limitations={["Traversal bound reached"]} />);
    expect(screen.getByText("Truncated result")).toBeInTheDocument();
    expect(screen.getByText("Traversal bound reached")).toBeInTheDocument();
  });
  it("provides accessible loading, empty and error states", () => {
    render(<><LoadingState label="Loading repository" /><EmptyState title="No repositories" description="Add one to continue." /><ErrorState description="Service unavailable." /></>);
    expect(screen.getByRole("status")).toHaveAttribute("aria-busy", "true");
    expect(screen.getByText("No repositories")).toBeInTheDocument();
    expect(screen.getByRole("alert")).toBeInTheDocument();
  });
});
