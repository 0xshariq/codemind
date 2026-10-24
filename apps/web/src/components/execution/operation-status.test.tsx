import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { OperationStatus } from "./operation-status";

describe("OperationStatus", () => {
  it.each([["completed", "Completed"], ["partial", "Partial"], ["failed", "Failed"], ["unavailable", "Unavailable"], ["planning", "In Progress"]])("maps %s to %s", (status, label) => {
    render(<OperationStatus status={status} />);
    expect(screen.getByText(label)).toBeInTheDocument();
  });
});
