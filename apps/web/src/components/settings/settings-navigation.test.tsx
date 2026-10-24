import { Settings } from "lucide-react";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SettingsNavigation } from "./settings-navigation";

describe("SettingsNavigation", () => {
  it("renders descriptive sections and selected state", () => {
    render(<SettingsNavigation activeId="general" items={[{ id: "general", title: "General", description: "Workspace defaults", icon: Settings }]} onSelect={() => undefined} />);
    expect(screen.getByRole("button", { name: /General Workspace defaults/i })).toHaveAttribute("aria-current", "location");
    expect(screen.getByText("Workspace defaults")).toBeInTheDocument();
  });
});
