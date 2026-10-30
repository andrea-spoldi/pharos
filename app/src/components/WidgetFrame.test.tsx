import { createMemoryHistory, MemoryRouter, Route } from "@solidjs/router";
import { fireEvent, render as renderUi, screen, waitFor } from "@solidjs/testing-library";
import { createSignal, type JSX } from "solid-js";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands, type WidgetData, type PlacementView, type WidgetSpec } from "../bindings";
import { WidgetFrame } from "./WidgetFrame";

vi.mock("../bindings", () => ({ commands: { runPlacement: vi.fn() } }));

window.scrollTo = vi.fn() as typeof window.scrollTo;

let history = createMemoryHistory();
/** The frame links to Settings, so it renders inside a router. */
const render = (ui: () => JSX.Element) => {
  history = createMemoryHistory();
  history.set({ value: "/d/home" });
  return renderUi(() => (
    <MemoryRouter history={history}>
      <Route path={["/d/:name", "/settings"]} component={ui} />
    </MemoryRouter>
  ));
};

const ok = <T,>(data: T) => ({ status: "ok" as const, data });

function placement(
  over: Partial<PlacementView>,
  source: WidgetSpec["source"] = { type: "file", path: "../../catalog/services/", glob: "*.yaml" },
  view: WidgetSpec["view"] = { type: "table", columns: [{ field: "metadata.name", label: "Service" }], content: null },
): PlacementView {
  return {
    widget: "service-catalog",
    x: 0,
    y: 0,
    w: 8,
    h: 4,
    manifest: {
      apiVersion: "pharos.io/v1alpha1",
      kind: "Widget",
      metadata: { name: "service-catalog", title: "Service Catalog", version: null, description: null },
      spec: { source, refresh: null, inputs: {}, view, permissions: { hosts: [] } },
    },
    status: { kind: "runnable" },
    reads: "catalog/services/*.yaml",
    ...over,
  };
}

const slot = { dashboard: "home", index: 2 };

const rows: WidgetData = { rows: [{ metadata: { name: "payment-service" }, _source: "p.yaml" }], truncated: false };

describe("WidgetFrame", () => {
  beforeEach(() => {
    vi.mocked(commands.runPlacement).mockReset().mockResolvedValue(ok(rows));
  });

  it("names a missing widget without running it", () => {
    render(() => <WidgetFrame slot={slot} placement={placement({ widget: "ghost", manifest: null, status: { kind: "missing" } })} version={0} />);
    const alert = screen.getByRole("alert");
    expect(alert.textContent).toContain("Missing widget");
    expect(alert.textContent).toContain("This dashboard places ghost, which the platform repo doesn't define.");
    expect(screen.getByText("ghost", { selector: "header *" })).toBeTruthy();
    expect(commands.runPlacement).not.toHaveBeenCalled();
  });

  it("lists what setup is needed", () => {
    render(() => (
      <WidgetFrame slot={slot}
        placement={placement({ status: { kind: "needsSetup", inputs: ["team"], secrets: ["GITLAB_TOKEN"] } })}
        version={0}
      />
    ));
    expect(screen.getByText("Needs setup")).toBeTruthy();
    expect(document.body.textContent).toContain("Needs your team · Needs secret GITLAB_TOKEN");
    expect(commands.runPlacement).not.toHaveBeenCalled();
  });

  it("names inputs by the platform's label when it has one", () => {
    render(() => (
      <WidgetFrame slot={slot}
        placement={placement({ status: { kind: "needsSetup", inputs: ["team", "region"], secrets: [] } })}
        version={0}
        inputLabel={(key) => (key === "team" ? "Your team" : null)}
      />
    ));
    expect(document.body.textContent).toContain("Needs Your team · Needs your region");
  });

  it("says what is not available yet", () => {
    render(() => <WidgetFrame slot={slot} placement={placement({ status: { kind: "notAvailable", feature: "command sources" } })} version={0} />);
    expect(screen.getByText("Not available yet")).toBeTruthy();
    expect(screen.getByText("command sources arrive in a later version.")).toBeTruthy();
    expect(commands.runPlacement).not.toHaveBeenCalled();
  });

  it("renders static markdown", () => {
    const p = placement({ status: { kind: "static" }, reads: null }, null, {
      type: "markdown",
      columns: [],
      content: "# Welcome to ACME",
    });
    const { container } = render(() => <WidgetFrame slot={slot} placement={p} version={0} />);
    expect(container.querySelector("h3")?.textContent).toBe("Welcome to ACME");
    expect(commands.runPlacement).not.toHaveBeenCalled();
  });

  it("shows a skeleton, then the data", async () => {
    render(() => <WidgetFrame slot={slot} placement={placement({})} version={0} />);
    expect(screen.getByLabelText("Loading").getAttribute("aria-busy")).toBe("true");
    expect(await screen.findByText("payment-service")).toBeTruthy();
    expect(screen.queryByLabelText("Loading")).toBeNull();
    expect(screen.getByText("Service Catalog")).toBeTruthy();
    expect(commands.runPlacement).toHaveBeenCalledWith("home", 2);
  });

  it("says where it looked when there are no rows", async () => {
    vi.mocked(commands.runPlacement).mockResolvedValue(ok({ rows: [], truncated: false }));
    render(() => <WidgetFrame slot={slot} placement={placement({})} version={0} />);
    await waitFor(() => expect(document.body.textContent).toContain("No data in catalog/services/*.yaml."));
  });

  it("explains a refused run with collapsed details", async () => {
    vi.mocked(commands.runPlacement).mockResolvedValue({
      status: "error",
      error: { kind: "permissionDenied", message: "path leaves the repository root" },
    });
    render(() => <WidgetFrame slot={slot} placement={placement({})} version={0} />);
    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("Access denied");
    expect(alert.querySelector("details summary")?.textContent).toBe("Details");
    expect(alert.querySelector("details pre")?.textContent).toBe("path leaves the repository root");
  });

  it("says the source was empty when it reads no files", async () => {
    vi.mocked(commands.runPlacement).mockResolvedValue(ok({ rows: [], truncated: false }));
    render(() => <WidgetFrame slot={slot} placement={placement({ reads: null })} version={0} />);
    await waitFor(() => expect(document.body.textContent).toContain("The source returned no rows."));
  });

  it("sends a refused token to Settings", async () => {
    vi.mocked(commands.runPlacement).mockResolvedValue({
      status: "error",
      error: { kind: "permissionDenied", message: "https://gitlab.com answered 401: {\"message\":\"401 Unauthorized\"}" },
    });
    const http: WidgetSpec["source"] = { type: "http", url: "https://gitlab.com/api/v4/projects" };
    render(() => <WidgetFrame slot={slot} placement={placement({ reads: null }, http)} version={0} />);
    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("Access denied");
    expect(alert.textContent).toContain("The service refused the request. Check the secrets this widget uses.");
    fireEvent.click(screen.getByRole("button", { name: "Open Settings" }));
    await waitFor(() => expect(history.get()).toBe("/settings"));
  });

  it("titles an unreachable service", async () => {
    vi.mocked(commands.runPlacement).mockResolvedValue({
      status: "error",
      error: { kind: "network", message: "request to https://gitlab.acme failed: timed out" },
    });
    render(() => <WidgetFrame slot={slot} placement={placement({})} version={0} />);
    expect((await screen.findByRole("alert")).textContent).toContain("Could not reach the service");
    expect(screen.queryByRole("button", { name: "Open Settings" })).toBeNull();
  });

  it("keeps data visible while re-running", async () => {
    const [version, setVersion] = createSignal(0);
    render(() => <WidgetFrame slot={slot} placement={placement({})} version={version()} />);
    await screen.findByText("payment-service");
    vi.mocked(commands.runPlacement).mockReturnValue(new Promise(() => {}));
    setVersion(1);
    await waitFor(() => expect(commands.runPlacement).toHaveBeenCalledTimes(2));
    expect(screen.getByText("payment-service")).toBeTruthy();
    expect(screen.queryByLabelText("Loading")).toBeNull();
  });

  it("needs setup links to the first missing field", async () => {
    render(() => (
      <WidgetFrame slot={slot}
        placement={placement({ status: { kind: "needsSetup", inputs: ["team"], secrets: ["GITLAB_TOKEN"] } })}
        version={0}
      />
    ));
    fireEvent.click(screen.getByRole("button", { name: "Open Settings" }));
    await waitFor(() => expect(history.get()).toBe("/settings?focus=input%3Ateam"));
  });

  it("links to the secret when no input is missing", async () => {
    render(() => (
      <WidgetFrame slot={slot} placement={placement({ status: { kind: "needsSetup", inputs: [], secrets: ["GITLAB_TOKEN"] } })} version={0} />
    ));
    fireEvent.click(screen.getByRole("button", { name: "Open Settings" }));
    await waitFor(() => expect(decodeURIComponent(history.get())).toBe("/settings?focus=secret:GITLAB_TOKEN"));
  });
});
