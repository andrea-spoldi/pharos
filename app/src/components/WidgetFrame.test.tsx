import { createMemoryHistory, MemoryRouter, Route } from "@solidjs/router";
import { fireEvent, render as renderUi, screen, waitFor } from "@solidjs/testing-library";
import { createSignal, type JSX } from "solid-js";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands, type WidgetData, type PlacementView, type WidgetSpec } from "../bindings";
import { WidgetFrame } from "./WidgetFrame";

vi.mock("../bindings", () => ({ commands: { runPlacement: vi.fn(), approveWidget: vi.fn() } }));

window.scrollTo = vi.fn() as typeof window.scrollTo;

let history = createMemoryHistory();
/** The frame links to Settings, so it renders inside a router. */
const render = (ui: () => JSX.Element) => {
  history = createMemoryHistory();
  history.set({ value: "/d/home" });
  return renderUi(() => (
    <MemoryRouter history={history}>
      <Route path={["/d/:id", "/settings"]} component={ui} />
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
    container: false,
    title: null,
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
    id: null,
    children: [],
    ...over,
  };
}

const slot = { dashboardId: "p:home", path: [2] };

const rows: WidgetData = { rows: [{ metadata: { name: "payment-service" }, _source: "p.yaml" }], truncated: false };

describe("WidgetFrame", () => {
  beforeEach(() => {
    vi.mocked(commands.runPlacement).mockReset().mockResolvedValue(ok(rows));
  });

  it("names a missing widget without running it", () => {
    render(() => <WidgetFrame onApproved={() => {}} slot={slot} placement={placement({ widget: "ghost", manifest: null, status: { kind: "missing" } })} version={0} />);
    const alert = screen.getByRole("alert");
    expect(alert.textContent).toContain("Missing widget");
    expect(alert.textContent).toContain("This dashboard places ghost, which the platform repo doesn't define.");
    expect(screen.getByText("ghost", { selector: "header *" })).toBeTruthy();
    expect(commands.runPlacement).not.toHaveBeenCalled();
  });

  it("lists what setup is needed", () => {
    render(() => (
      <WidgetFrame onApproved={() => {}} slot={slot}
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
      <WidgetFrame onApproved={() => {}} slot={slot}
        placement={placement({ status: { kind: "needsSetup", inputs: ["team", "region"], secrets: [] } })}
        version={0}
        inputLabel={(key) => (key === "team" ? "Your team" : null)}
      />
    ));
    expect(document.body.textContent).toContain("Needs Your team · Needs your region");
  });

  it("says what is not available yet", () => {
    render(() => <WidgetFrame onApproved={() => {}} slot={slot} placement={placement({ status: { kind: "notAvailable", feature: "command sources" } })} version={0} />);
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
    const { container } = render(() => <WidgetFrame onApproved={() => {}} slot={slot} placement={p} version={0} />);
    expect(container.querySelector("h3")?.textContent).toBe("Welcome to ACME");
    expect(commands.runPlacement).not.toHaveBeenCalled();
  });

  it("shows a skeleton, then the data", async () => {
    render(() => <WidgetFrame onApproved={() => {}} slot={slot} placement={placement({})} version={0} />);
    expect(screen.getByLabelText("Loading").getAttribute("aria-busy")).toBe("true");
    expect(await screen.findByText("payment-service")).toBeTruthy();
    expect(screen.queryByLabelText("Loading")).toBeNull();
    expect(screen.getByText("Service Catalog")).toBeTruthy();
    expect(commands.runPlacement).toHaveBeenCalledWith("p:home", [2]);
  });

  it("says where it looked when there are no rows", async () => {
    vi.mocked(commands.runPlacement).mockResolvedValue(ok({ rows: [], truncated: false }));
    render(() => <WidgetFrame onApproved={() => {}} slot={slot} placement={placement({})} version={0} />);
    await waitFor(() => expect(document.body.textContent).toContain("No data in catalog/services/*.yaml."));
  });

  it("explains a refused run with collapsed details", async () => {
    vi.mocked(commands.runPlacement).mockResolvedValue({
      status: "error",
      error: { kind: "permissionDenied", message: "path leaves the repository root" },
    });
    render(() => <WidgetFrame onApproved={() => {}} slot={slot} placement={placement({})} version={0} />);
    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("Access denied");
    expect(alert.querySelector("details summary")?.textContent).toBe("Details");
    expect(alert.querySelector("details pre")?.textContent).toBe("path leaves the repository root");
  });

  it("says the source was empty when it reads no files", async () => {
    vi.mocked(commands.runPlacement).mockResolvedValue(ok({ rows: [], truncated: false }));
    render(() => <WidgetFrame onApproved={() => {}} slot={slot} placement={placement({ reads: null })} version={0} />);
    await waitFor(() => expect(document.body.textContent).toContain("The source returned no rows."));
  });

  it("sends a refused token to Settings", async () => {
    vi.mocked(commands.runPlacement).mockResolvedValue({
      status: "error",
      error: { kind: "permissionDenied", message: "https://gitlab.com answered 401: {\"message\":\"401 Unauthorized\"}" },
    });
    const http: WidgetSpec["source"] = { type: "http", url: "https://gitlab.com/api/v4/projects" };
    render(() => <WidgetFrame onApproved={() => {}} slot={slot} placement={placement({ reads: null }, http)} version={0} />);
    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("Access denied");
    expect(alert.textContent).toContain("The service refused the request. Check the secrets this widget uses.");
    fireEvent.click(screen.getByRole("button", { name: "Open Settings" }));
    await waitFor(() => expect(history.get()).toBe("/settings"));
  });

  it("sends a program that was not found to the search path in Settings", async () => {
    vi.mocked(commands.runPlacement).mockResolvedValue({
      status: "error",
      error: { kind: "executableNotFound", message: "`kubectl` was not found in /usr/bin, /bin" },
    });
    const cmd: WidgetSpec["source"] = { type: "command", executable: "kubectl", args: [], timeout: null };
    render(() => <WidgetFrame onApproved={() => {}} slot={slot} placement={placement({ reads: null }, cmd)} version={0} />);
    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("Program not found");
    expect(alert.textContent).toContain("Add the folder that contains it to the program search path in Settings.");
    fireEvent.click(screen.getByRole("button", { name: "Open Settings" }));
    await waitFor(() => expect(history.get()).toBe("/settings?focus=search-path"));
  });

  it("shows a failed command's stderr without a Details toggle", async () => {
    vi.mocked(commands.runPlacement).mockResolvedValue({
      status: "error",
      error: { kind: "commandFailed", message: "`kubectl` exited with code 1: error: current-context is not set" },
    });
    render(() => <WidgetFrame onApproved={() => {}} slot={slot} placement={placement({})} version={0} />);
    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("The command failed");
    expect(alert.querySelector("details")).toBeNull();
    expect(alert.querySelector("pre")?.textContent).toContain("current-context is not set");
  });

  it("titles an unreachable service", async () => {
    vi.mocked(commands.runPlacement).mockResolvedValue({
      status: "error",
      error: { kind: "network", message: "request to https://gitlab.acme failed: timed out" },
    });
    render(() => <WidgetFrame onApproved={() => {}} slot={slot} placement={placement({})} version={0} />);
    expect((await screen.findByRole("alert")).textContent).toContain("Could not reach the service");
    expect(screen.queryByRole("button", { name: "Open Settings" })).toBeNull();
  });

  it("keeps data visible while re-running", async () => {
    const [version, setVersion] = createSignal(0);
    render(() => <WidgetFrame onApproved={() => {}} slot={slot} placement={placement({})} version={version()} />);
    await screen.findByText("payment-service");
    vi.mocked(commands.runPlacement).mockReturnValue(new Promise(() => {}));
    setVersion(1);
    await waitFor(() => expect(commands.runPlacement).toHaveBeenCalledTimes(2));
    expect(screen.getByText("payment-service")).toBeTruthy();
    expect(screen.queryByLabelText("Loading")).toBeNull();
  });

  it("needs setup links to the first missing field", async () => {
    render(() => (
      <WidgetFrame onApproved={() => {}} slot={slot}
        placement={placement({ status: { kind: "needsSetup", inputs: ["team"], secrets: ["GITLAB_TOKEN"] } })}
        version={0}
      />
    ));
    fireEvent.click(screen.getByRole("button", { name: "Open Settings" }));
    await waitFor(() => expect(history.get()).toBe("/settings?focus=input%3Ateam"));
  });

  it("links to the secret when no input is missing", async () => {
    render(() => (
      <WidgetFrame onApproved={() => {}} slot={slot} placement={placement({ status: { kind: "needsSetup", inputs: [], secrets: ["GITLAB_TOKEN"] } })} version={0} />
    ));
    fireEvent.click(screen.getByRole("button", { name: "Open Settings" }));
    await waitFor(() => expect(decodeURIComponent(history.get())).toBe("/settings?focus=secret:GITLAB_TOKEN"));
  });

  it("renders_container_children_as_subgrid", async () => {
    const child = (name: string, path: number[]) =>
      placement({
        widget: name,
        manifest: {
          apiVersion: "pharos.io/v1alpha1",
          kind: "Widget",
          metadata: { name, title: name.toUpperCase(), version: null, description: null },
          spec: { source: null, refresh: null, inputs: {}, view: { type: "markdown", content: `hi ${name}` }, permissions: { hosts: [] } },
        },
        status: { kind: "static" },
        reads: null,
        w: 6,
        h: 2,
        x: path[0] * 6,
      });
    render(() => (
      <WidgetFrame
        onApproved={() => {}}
        slot={{ dashboardId: "p:home", path: [3] }}
        placement={placement({
          widget: "pair",
          status: { kind: "runnable" },
          manifest: {
            apiVersion: "pharos.io/v1alpha1",
            kind: "Widget",
            metadata: { name: "pair", title: "Pair", version: null, description: null },
            spec: { source: null, refresh: null, inputs: {}, view: null, permissions: { hosts: [] } },
          },
          container: true,
          children: [child("one", [0]), child("two", [1])],
        })}
        version={0}
      />
    ));
    const sub = screen.getByRole("region", { name: "Pair" });
    expect(sub.querySelectorAll("article.widget").length).toBe(2);
    expect(screen.getByText("ONE", { selector: "header *" })).toBeTruthy();
    expect(screen.getByText("TWO", { selector: "header *" })).toBeTruthy();
    // The container itself is not run, nor shows the leaf status switch.
    expect(commands.runPlacement).not.toHaveBeenCalled();
  });

  it("runs container children by parent path plus child index", async () => {
    const leaf = (name: string) => placement({ widget: name });
    render(() => (
      <WidgetFrame
        onApproved={() => {}}
        slot={{ dashboardId: "p:home", path: [3] }}
        placement={placement({ widget: "pair", container: true, children: [leaf("a"), leaf("b")] })}
        version={0}
      />
    ));
    await waitFor(() => expect(commands.runPlacement).toHaveBeenCalledTimes(2));
    expect(vi.mocked(commands.runPlacement).mock.calls).toEqual([
      ["p:home", [3, 0]],
      ["p:home", [3, 1]],
    ]);
  });

  it("says so when a leaf has no view", () => {
    render(() => (
      <WidgetFrame
        onApproved={() => {}}
        slot={slot}
        placement={placement({ status: { kind: "static" } }, null, null)}
        version={0}
      />
    ));
    expect(screen.getByText("No view")).toBeTruthy();
  });

  describe("user groups", () => {
    const edit = () => ({ editable: true, onAdd: vi.fn(), onRename: vi.fn(), onRemove: vi.fn() });
    const group = (over: Partial<PlacementView>) =>
      placement({ widget: "group", container: true, title: "Ops", manifest: null, status: { kind: "static" }, reads: null, id: 7, ...over });

    it("renders an empty group as a titled sub-grid with an Add widget control", () => {
      const e = edit();
      render(() => <WidgetFrame onApproved={() => {}} slot={slot} placement={group({ children: [] })} version={0} edit={e} />);
      const sub = screen.getByRole("region", { name: "Ops" });
      expect(sub.classList.contains("subgrid")).toBe(true);
      expect(screen.getByText("Ops", { selector: "header *" })).toBeTruthy();
      fireEvent.click(screen.getByRole("button", { name: "Add widget to Ops" }));
      expect(e.onAdd).toHaveBeenCalledWith(7);
      expect(screen.queryByText("No view")).toBeNull();
    });

    it("offers no group controls when not editing", () => {
      render(() => <WidgetFrame onApproved={() => {}} slot={slot} placement={group({ children: [] })} version={0} />);
      expect(screen.getByRole("region", { name: "Ops" })).toBeTruthy();
      expect(screen.queryByRole("button", { name: /Add widget/ })).toBeNull();
      expect(screen.queryByRole("button", { name: /Rename/ })).toBeNull();
    });

    it("wires rename and remove on the group, and remove on a member", () => {
      const e = edit();
      const member = placement({ widget: "welcome", manifest: null, status: { kind: "missing" }, reads: null, id: 9 });
      render(() => <WidgetFrame onApproved={() => {}} slot={slot} placement={group({ children: [member] })} version={0} edit={e} />);
      const sub = screen.getByRole("region", { name: "Ops" });
      expect(sub.querySelectorAll("article.widget").length).toBe(1);
      fireEvent.click(screen.getByRole("button", { name: "Remove welcome" }));
      expect(e.onRemove).toHaveBeenCalledWith(9, "welcome");
      fireEvent.click(screen.getByRole("button", { name: "Rename Ops" }));
      expect(e.onRename).toHaveBeenCalledWith(7, "Ops");
      fireEvent.click(screen.getByRole("button", { name: "Remove Ops" }));
      expect(e.onRemove).toHaveBeenCalledWith(7, "Ops");
    });
  });

  describe("needs approval", () => {
    const needs: PlacementView["status"] = { kind: "needsApproval", added: { hosts: ["https://api.example.com"] } };
    const blocked = (over: Partial<PlacementView> = {}) =>
      placement({ status: needs, ...over });

    beforeEach(() => {
      vi.mocked(commands.approveWidget).mockReset().mockResolvedValue(ok(null));
    });

    it("shows the added capabilities and does not run", () => {
      render(() => <WidgetFrame slot={slot} placement={blocked()} version={0} onApproved={() => {}} />);
      expect(screen.getByText("Needs approval")).toBeTruthy();
      expect(screen.getByText("This widget now wants to:")).toBeTruthy();
      expect(document.body.textContent).toContain("api.example.com");
      expect(commands.runPlacement).not.toHaveBeenCalled();
    });

    it("Approve calls approveWidget with the widget name then onApproved", async () => {
      const onApproved = vi.fn();
      render(() => <WidgetFrame slot={slot} placement={blocked()} version={0} onApproved={onApproved} />);
      fireEvent.click(screen.getByRole("button", { name: "Approve" }));
      await waitFor(() => expect(onApproved).toHaveBeenCalledTimes(1));
      expect(commands.approveWidget).toHaveBeenCalledWith("service-catalog");
    });

    it("a failed approve shows the error", async () => {
      vi.mocked(commands.approveWidget).mockResolvedValue({ status: "error", error: { kind: "internal", message: "boom" } });
      const onApproved = vi.fn();
      render(() => <WidgetFrame slot={slot} placement={blocked()} version={0} onApproved={onApproved} />);
      fireEvent.click(screen.getByRole("button", { name: "Approve" }));
      const alert = await screen.findByRole("alert");
      expect(alert.textContent).toContain("boom");
      expect(onApproved).not.toHaveBeenCalled();
    });

    it("a container with needsApproval renders its children and has no Approve of its own", () => {
      const child = blocked({ widget: "child", title: "Child" });
      render(() => (
        <WidgetFrame
          slot={slot}
          placement={blocked({ container: true, title: "Group", children: [child] })}
          version={0}
          onApproved={() => {}}
        />
      ));
      expect(screen.getAllByRole("button", { name: "Approve" })).toHaveLength(1);
      expect(screen.getAllByText("Needs approval")).toHaveLength(1);
    });
  });
});
