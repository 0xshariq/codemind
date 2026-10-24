import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ImpactCard } from "./impact-card";
import { RiskCard } from "./risk-card";
import { TestImpactPanel } from "./test-impact-panel";
import { RecommendationsPanel } from "./recommendations-panel";

const path = { id: "p1", label: "caller", classification: "direct", depth: 1 };
describe("analysis result components", () => {
  it("separates impact and risk semantics", () => {
    render(<><ImpactCard directCount={1} indirectCount={2} paths={[path]} completeness="complete" /><RiskCard risk="unknown" unknownFactors={["Dependency history unavailable"]} /></>);
    expect(screen.getByText("Impact Analysis")).toBeInTheDocument();
    expect(screen.getByText("Risk Assessment")).toBeInTheDocument();
    expect(screen.getByText("Dependency history unavailable")).toBeInTheDocument();
  });
  it("does not present a test candidate as executed", () => {
    render(<TestImpactPanel tests={[{ id: "t1", name: "auth_test", path: "tests/auth.rs" }]} />);
    expect(screen.getByText("Candidate")).toBeInTheDocument();
    expect(screen.getByText(/not execution results/i)).toBeInTheDocument();
  });
  it("renders an explicit empty recommendations state", () => {
    render(<RecommendationsPanel recommendations={[]} />);
    expect(screen.getByText("No recommendations")).toBeInTheDocument();
  });
});
