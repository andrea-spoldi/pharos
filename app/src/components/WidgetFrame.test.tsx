import { render, screen, waitFor } from "@solidjs/testing-library";
import { createSignal } from "solid-js";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands, type FileData, type PlacementView, type WidgetSpec } from "../bindings";
import { WidgetFrame } from "./WidgetFrame";

vi.mock("../bindings", () => ({ commands: { runWidget: vi.fn() } }));

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
      apiVersion: "cockpit.dev/v1alpha1",
      kind: "Widget",
      metadata: { name: "service-catalog", title: "Service Catalog", version: null, description: null },
      spec: { source, refresh: null, inputs: {}, view, permissions: { hosts: [] } },
    },
    status: { kind: "runnable" },
    reads: "catalog/services/*.yaml",
    ...over,
  };
}

const rows: FileData = { rows: [{ metadata: { name: "payment-service" }, _source: "p.yaml" }], truncated: false };

describe("WidgetFrame", () => {
  beforeEach(() => {
    vi.mocked(commands.runWidget).mockReset().mockResolvedValue(ok(rows));
  });

  it("names a missing widget without running it", () => {
    render(() => <WidgetFrame placement={placement({ widget: "ghost", manifest: null, status: { kind: "missing" } })} version={0} />);
    const alert = screen.getByRole("alert");
    expect(alert.textContent).toContain("Missing widget");
    expect(alert.textContent).toContain("This dashboard places ghost, which the platform repo doesn't define.");
    expect(screen.getByText("ghost", { selector: "header *" })).toBeTruthy();
    expect(commands.runWidget).not.toHaveBeenCalled();
  });

  it("lists what setup is needed", () => {
    render(() => (
      <WidgetFrame
        placement={placement({ status: { kind: "needsSetup", inputs: ["team"], secrets: ["GITLAB_TOKEN"] } })}
        version={0}
      />
    ));
    expect(screen.getByText("Needs setup")).toBeTruthy();
    expect(document.body.textContent).toContain("Needs your team · Needs secret GITLAB_TOKEN");
    expect(commands.runWidget).not.toHaveBeenCalled();
  });

  it("says what is not available yet", () => {
    render(() => <WidgetFrame placement={placement({ status: { kind: "notAvailable", feature: "command sources" } })} version={0} />);
    expect(screen.getByText("Not available yet")).toBeTruthy();
    expect(screen.getByText("command sources arrive in a later version.")).toBeTruthy();
    expect(commands.runWidget).not.toHaveBeenCalled();
  });

  it("renders static markdown", () => {
    const p = placement({ status: { kind: "static" }, reads: null }, null, {
      type: "markdown",
      columns: [],
      content: "# Welcome to ACME",
    });
    const { container } = render(() => <WidgetFrame placement={p} version={0} />);
    expect(container.querySelector("h3")?.textContent).toBe("Welcome to ACME");
    expect(commands.runWidget).not.toHaveBeenCalled();
  });

  it("shows a skeleton, then the data", async () => {
    render(() => <WidgetFrame placement={placement({})} version={0} />);
    expect(screen.getByLabelText("Loading").getAttribute("aria-busy")).toBe("true");
    expect(await screen.findByText("payment-service")).toBeTruthy();
    expect(screen.queryByLabelText("Loading")).toBeNull();
    expect(screen.getByText("Service Catalog")).toBeTruthy();
    expect(commands.runWidget).toHaveBeenCalledWith("service-catalog");
  });

  it("says where it looked when there are no rows", async () => {
    vi.mocked(commands.runWidget).mockResolvedValue(ok({ rows: [], truncated: false }));
    render(() => <WidgetFrame placement={placement({})} version={0} />);
    await waitFor(() => expect(document.body.textContent).toContain("No data in catalog/services/*.yaml."));
  });

  it("explains a refused run with collapsed details", async () => {
    vi.mocked(commands.runWidget).mockResolvedValue({
      status: "error",
      error: { kind: "permissionDenied", message: "path leaves the repository root" },
    });
    render(() => <WidgetFrame placement={placement({})} version={0} />);
    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("Access denied");
    expect(alert.querySelector("details summary")?.textContent).toBe("Details");
    expect(alert.querySelector("details pre")?.textContent).toBe("path leaves the repository root");
  });

  it("keeps data visible while re-running", async () => {
    const [version, setVersion] = createSignal(0);
    render(() => <WidgetFrame placement={placement({})} version={version()} />);
    await screen.findByText("payment-service");
    vi.mocked(commands.runWidget).mockReturnValue(new Promise(() => {}));
    setVersion(1);
    await waitFor(() => expect(commands.runWidget).toHaveBeenCalledTimes(2));
    expect(screen.getByText("payment-service")).toBeTruthy();
    expect(screen.queryByLabelText("Loading")).toBeNull();
  });
});
