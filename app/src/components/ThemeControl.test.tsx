import { fireEvent, render, screen } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it } from "vitest";
import { ThemeControl } from "./ThemeControl";
import { setTheme } from "../theme";

describe("ThemeControl", () => {
  beforeEach(() => {
    localStorage.clear();
    setTheme("system");
  });

  it("offers Light, Dark and System and marks the current one", () => {
    render(() => <ThemeControl />);
    expect(screen.getAllByRole("radio")).toHaveLength(3);
    expect((screen.getByRole("radio", { name: "System" }) as HTMLInputElement).checked).toBe(true);
    expect((screen.getByRole("radio", { name: "Dark" }) as HTMLInputElement).checked).toBe(false);
  });

  it("applies the chosen theme to <html> and persists it", () => {
    render(() => <ThemeControl />);
    fireEvent.click(screen.getByRole("radio", { name: "Dark" }));
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(localStorage.getItem("pharos.theme")).toBe("dark");
    const dark = screen.getByRole("radio", { name: "Dark" }) as HTMLInputElement;
    expect(dark.checked).toBe(true);
    // The selected segment carries the highlight class.
    expect(dark.closest("label")?.classList.contains("selected")).toBe(true);
  });
});
