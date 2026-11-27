import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

function setWidth(w: number) {
  Object.defineProperty(window, "innerWidth", { configurable: true, writable: true, value: w });
}

describe("sidebar state", () => {
  beforeEach(() => {
    vi.resetModules();
    localStorage.clear();
    setWidth(1440);
  });
  afterEach(() => {
    vi.restoreAllMocks();
    setWidth(1024);
  });

  it("default_is_open_at_1280_and_rail_below", async () => {
    const { defaultMode } = await import("./sidebar");
    expect(defaultMode(1280)).toBe("open");
    expect(defaultMode(1279)).toBe("rail");
  });

  it("saved_choice_wins_over_width", async () => {
    localStorage.setItem("pharos.sidebar", "rail");
    const { sidebarMode } = await import("./sidebar");
    expect(sidebarMode()).toBe("rail");
  });

  it("toggle_persists", async () => {
    const { sidebarMode, toggleSidebar, SIDEBAR_KEY } = await import("./sidebar");
    expect(sidebarMode()).toBe("open");
    toggleSidebar();
    expect(sidebarMode()).toBe("rail");
    expect(localStorage.getItem(SIDEBAR_KEY)).toBe("rail");
  });

  it("mode_survives_unavailable_storage", async () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    setWidth(1000);
    const { sidebarMode, toggleSidebar } = await import("./sidebar");
    expect(sidebarMode()).toBe("rail");
    toggleSidebar();
    expect(sidebarMode()).toBe("open");
  });

  it("narrow_tracks_resize", async () => {
    setWidth(1000);
    const { narrow } = await import("./viewport");
    expect(narrow()).toBe(false);
    setWidth(800);
    window.dispatchEvent(new Event("resize"));
    expect(narrow()).toBe(true);
    setWidth(1000);
    window.dispatchEvent(new Event("resize"));
    expect(narrow()).toBe(false);
  });
});
