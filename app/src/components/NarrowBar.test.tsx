import { fireEvent, render, screen, within } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import type { DashboardView } from "../bindings";
import { NarrowBar } from "./NarrowBar";

function view(id: string, title: string): DashboardView {
  const user = id.startsWith("u:");
  return {
    id,
    origin: user ? "user" : "platform",
    editable: user,
    metadata: { name: title.toLowerCase(), title, version: null, description: null },
    placements: [],
  };
}

const list = [view("p:home", "Home"), view("p:ops", "Ops"), view("u:1", "My work")];
const base = { dashboards: list, title: "ACME", status: null, syncing: false, onSettings: () => {}, onOpen: () => {} };
const trigger = () => screen.getByRole("button", { name: /Home/ });

describe("NarrowBar", () => {
  it("narrow_bar_menu_lists_both_groups", () => {
    render(() => <NarrowBar {...base} current="p:home" onSync={() => {}} />);
    const button = trigger();
    expect(button.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(button);
    expect(button.getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByRole("heading", { name: "Platform" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Mine" })).toBeTruthy();
    const menu = within(document.querySelector(".menu") as HTMLElement);
    expect(menu.getAllByRole("button").map((b) => b.textContent)).toEqual([
      "Home",
      "Ops",
      "My work",
      "Sync now",
      "Settings",
    ]);
    expect(menu.getByRole("button", { name: "Home" }).getAttribute("aria-current")).toBe("page");
  });

  it("menu_closes_on_escape_and_selection", () => {
    const onOpen = vi.fn();
    render(() => <NarrowBar {...base} current="p:home" onOpen={onOpen} />);
    const button = trigger();
    fireEvent.click(button);
    fireEvent.keyDown(screen.getByRole("button", { name: "Ops" }), { key: "Escape" });
    expect(document.querySelector(".menu")).toBeNull();
    expect(document.activeElement).toBe(button);

    fireEvent.click(button);
    fireEvent.click(screen.getByRole("button", { name: "Ops" }));
    expect(onOpen).toHaveBeenCalledWith("p:ops");
    expect(document.querySelector(".menu")).toBeNull();
    expect(document.activeElement).toBe(button);
  });

  it("falls_back_to_the_platform_title", () => {
    render(() => <NarrowBar {...base} dashboards={[]} />);
    expect(screen.getByRole("button", { name: /ACME/ })).toBeTruthy();
  });
});
