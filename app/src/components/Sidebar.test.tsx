import { fireEvent, render, screen, waitFor, within } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands, type DashboardView } from "../bindings";
import { SIDEBAR_KEY, setSidebarMode, sidebarMode } from "../sidebar";
import { repoName, Sidebar } from "./Sidebar";

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

const chrome = {
  title: "ACME",
  source: {
    id: 1,
    url: "https://github.com/acme/platform.git",
    branch: "main",
    local_path: "",
    synced_commit: "a".repeat(40),
    synced_at: null,
    approved_capability_hash: null,
  },
  status: null,
  syncing: false,
  onSettings: () => {},
  settingsCurrent: false,
};
const list = [view("p:home", "Home"), view("p:ops", "Ops"), view("u:1", "My work")];
const platformNav = () => screen.getByRole("navigation", { name: "Platform dashboards" });
const mineNav = () => screen.getByRole("navigation", { name: "Your dashboards" });
const labels = (nav: HTMLElement) =>
  within(nav)
    .getAllByRole("button")
    .map((b) => b.querySelector(".nav-label")?.textContent);

describe("Sidebar", () => {
  beforeEach(() => {
    vi.mocked(commands.createDashboard).mockReset();
    localStorage.clear();
    setSidebarMode("open");
  });

  it("groups_platform_and_mine", () => {
    render(() => <Sidebar {...chrome} dashboards={list} onOpen={() => {}} onCreated={() => {}} />);
    expect(within(platformNav()).getByRole("heading", { name: "Platform" })).toBeTruthy();
    expect(within(mineNav()).getByRole("heading", { name: "Mine" })).toBeTruthy();
    expect(labels(platformNav())).toEqual(["Home", "Ops"]);
    expect(within(mineNav()).getByRole("button", { name: "My work" })).toBeTruthy();
    expect(within(mineNav()).queryByRole("button", { name: "Home" })).toBeNull();
  });

  it("current_item_has_aria_current", () => {
    const onOpen = vi.fn();
    render(() => <Sidebar {...chrome} dashboards={list} current="p:ops" onOpen={onOpen} onCreated={() => {}} />);
    expect(screen.getByRole("button", { name: "Ops" }).getAttribute("aria-current")).toBe("page");
    expect(screen.getByRole("button", { name: "Home" }).getAttribute("aria-current")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "My work" }));
    expect(onOpen).toHaveBeenCalledWith("u:1");
  });

  it("landing_cue_in_accessible_name", () => {
    render(() => <Sidebar {...chrome} dashboards={list} landing="p:home" onOpen={() => {}} onCreated={() => {}} />);
    const home = screen.getByRole("button", { name: /Home.*Landing dashboard/ });
    expect(home.querySelector(".landing-mark")?.getAttribute("aria-hidden")).toBe("true");
    expect(screen.getByRole("button", { name: "Ops" })).toBeTruthy();
  });

  it("new_dashboard_lives_under_mine", async () => {
    const created = view("u:2", "Fresh");
    vi.mocked(commands.createDashboard).mockResolvedValue(ok(created));
    const onCreated = vi.fn();
    render(() => <Sidebar {...chrome} dashboards={list} onOpen={() => {}} onCreated={onCreated} />);
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

  it("derives_owner_repo_with_fallback_to_the_full_url", () => {
    expect(repoName("https://github.com/acme/platform.git")).toBe("acme/platform");
    expect(repoName("git@github.com:acme/platform.git")).toBe("acme/platform");
    expect(repoName("/Users/me/work/platform/")).toBe("work/platform");
    expect(repoName("platform")).toBe("platform");
  });

  it("footer_offers_sync_and_settings", () => {
    const onSync = vi.fn();
    const onSettings = vi.fn();
    render(() => (
      <Sidebar
        {...chrome}
        dashboards={list}
        onOpen={() => {}}
        onCreated={() => {}}
        onSync={onSync}
        onSettings={onSettings}
        settingsCurrent
      />
    ));
    fireEvent.click(screen.getByRole("button", { name: "Sync now" }));
    expect(onSync).toHaveBeenCalled();
    const settings = screen.getByRole("button", { name: "Settings" });
    expect(settings.getAttribute("aria-current")).toBe("page");
    fireEvent.click(settings);
    expect(onSettings).toHaveBeenCalled();
  });

  describe("rail", () => {
    const mount = (extra: Record<string, unknown> = {}) =>
      render(() => (
        <Sidebar {...chrome} dashboards={list} onOpen={() => {}} onCreated={() => {}} onSync={() => {}} {...extra} />
      ));

    it("toggle_sets_aria_expanded_and_persists", () => {
      mount();
      const toggle = screen.getByRole("button", { name: "Collapse sidebar" });
      expect(toggle.getAttribute("aria-expanded")).toBe("true");
      fireEvent.click(toggle);
      const expand = screen.getByRole("button", { name: "Expand sidebar" });
      expect(expand.getAttribute("aria-expanded")).toBe("false");
      expect(localStorage.getItem(SIDEBAR_KEY)).toBe("rail");
    });

    it("rail_labels_reachable_by_accessible_name", () => {
      setSidebarMode("rail");
      mount();
      expect(screen.getByRole("button", { name: "Home" })).toBeTruthy();
      expect(screen.getByRole("button", { name: "Settings" })).toBeTruthy();
      expect(screen.getByRole("button", { name: "Sync now" }).textContent).toBe("↻");
      expect(screen.getByRole("button", { name: "+ New dashboard" })).toBeTruthy();
      expect(screen.getByRole("heading", { name: "Platform" })).toBeTruthy();
      expect(screen.getByTestId("brand").textContent).toContain("ACME");
      expect(screen.getByTestId("brand").querySelector(".sr-only")).toBeTruthy();
      expect(screen.getByTestId("source").classList.contains("sr-only")).toBe(true);
    });

    it("rail_keeps_sync_status_region", () => {
      setSidebarMode("rail");
      mount({ status: "Synced" });
      const region = screen.getByRole("status");
      expect(region.textContent).toBe("Synced");
      expect(region.classList.contains("sr-only")).toBe(true);
    });

    it("shortcut_toggles", () => {
      mount();
      fireEvent.keyDown(document.body, { key: "\\", ctrlKey: true });
      expect(sidebarMode()).toBe("rail");
      fireEvent.keyDown(document.body, { key: "\\", metaKey: true });
      expect(sidebarMode()).toBe("open");
      fireEvent.keyDown(document.body, { key: "\\" });
      expect(sidebarMode()).toBe("open");
    });

    it("shortcut_ignored_while_typing_or_in_dialog", () => {
      mount();
      const input = document.createElement("input");
      document.body.append(input);
      input.focus();
      fireEvent.keyDown(input, { key: "\\", ctrlKey: true });
      expect(sidebarMode()).toBe("open");
      input.remove();

      fireEvent.click(screen.getByRole("button", { name: "+ New dashboard" }));
      expect(screen.getByRole("dialog")).toBeTruthy();
      fireEvent.keyDown(document.body, { key: "\\", ctrlKey: true });
      expect(sidebarMode()).toBe("open");
    });

    it("tooltip_on_focus_in_rail", () => {
      setSidebarMode("rail");
      mount();
      expect(screen.queryByRole("tooltip")).toBeNull();
      fireEvent.focusIn(screen.getByRole("button", { name: "Ops" }));
      expect(screen.getByRole("tooltip").textContent).toBe("Ops");
      fireEvent.focusOut(screen.getByRole("button", { name: "Ops" }));
      expect(screen.queryByRole("tooltip")).toBeNull();
      fireEvent.mouseOver(screen.getByRole("button", { name: "Sync now" }));
      expect(screen.getByRole("tooltip").textContent).toBe("Sync now");
    });

    it("tooltip_clears_when_leaving_rail", () => {
      setSidebarMode("rail");
      mount();
      fireEvent.focusIn(screen.getByRole("button", { name: "Ops" }));
      expect(screen.getByRole("tooltip")).toBeTruthy();
      setSidebarMode("open");
      expect(screen.queryByRole("tooltip")).toBeNull();
    });

    it("no_tooltip_when_open", () => {
      mount();
      fireEvent.focusIn(screen.getByRole("button", { name: "Ops" }));
      expect(screen.queryByRole("tooltip")).toBeNull();
    });
  });
});
