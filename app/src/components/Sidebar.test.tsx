import { fireEvent, render, screen, waitFor, within } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands, type DashboardView } from "../bindings";
import { Sidebar } from "./Sidebar";

vi.mock("../bindings", () => ({ commands: { createDashboard: vi.fn() } }));

const ok = <T,>(data: T) => ({ status: "ok" as const, data });

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
const platformNav = () => screen.getByRole("navigation", { name: "Platform dashboards" });
const mineNav = () => screen.getByRole("navigation", { name: "Your dashboards" });
const labels = (nav: HTMLElement) =>
  within(nav)
    .getAllByRole("button")
    .map((b) => b.querySelector(".nav-label")?.textContent);

describe("Sidebar", () => {
  beforeEach(() => vi.mocked(commands.createDashboard).mockReset());

  it("groups_platform_and_mine", () => {
    render(() => <Sidebar dashboards={list} onOpen={() => {}} onCreated={() => {}} />);
    expect(within(platformNav()).getByRole("heading", { name: "Platform" })).toBeTruthy();
    expect(within(mineNav()).getByRole("heading", { name: "Mine" })).toBeTruthy();
    expect(labels(platformNav())).toEqual(["Home", "Ops"]);
    expect(within(mineNav()).getByRole("button", { name: "My work" })).toBeTruthy();
    expect(within(mineNav()).queryByRole("button", { name: "Home" })).toBeNull();
  });

  it("current_item_has_aria_current", () => {
    const onOpen = vi.fn();
    render(() => <Sidebar dashboards={list} current="p:ops" onOpen={onOpen} onCreated={() => {}} />);
    expect(screen.getByRole("button", { name: "Ops" }).getAttribute("aria-current")).toBe("page");
    expect(screen.getByRole("button", { name: "Home" }).getAttribute("aria-current")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "My work" }));
    expect(onOpen).toHaveBeenCalledWith("u:1");
  });

  it("landing_cue_in_accessible_name", () => {
    render(() => <Sidebar dashboards={list} landing="p:home" onOpen={() => {}} onCreated={() => {}} />);
    const home = screen.getByRole("button", { name: /Home.*Landing dashboard/ });
    expect(home.querySelector(".landing-mark")?.getAttribute("aria-hidden")).toBe("true");
    expect(screen.getByRole("button", { name: "Ops" })).toBeTruthy();
  });

  it("new_dashboard_lives_under_mine", async () => {
    const created = view("u:2", "Fresh");
    vi.mocked(commands.createDashboard).mockResolvedValue(ok(created));
    const onCreated = vi.fn();
    render(() => <Sidebar dashboards={list} onOpen={() => {}} onCreated={onCreated} />);
    expect(within(mineNav()).getAllByRole("button").at(-1)).toBe(
      within(mineNav()).getByRole("button", { name: "+ New dashboard" }),
    );
    expect(within(platformNav()).queryByRole("button", { name: "+ New dashboard" })).toBeNull();

    fireEvent.click(within(mineNav()).getByRole("button", { name: "+ New dashboard" }));
    const dialog = within(screen.getByRole("dialog"));
    fireEvent.input(dialog.getByLabelText("Title"), { target: { value: "Fresh" } });
    fireEvent.click(dialog.getByRole("button", { name: "Create" }));
    await waitFor(() => expect(onCreated).toHaveBeenCalledWith(created));
    expect(commands.createDashboard).toHaveBeenCalledWith("Fresh");
  });
});
