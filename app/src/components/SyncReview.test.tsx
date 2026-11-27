import { fireEvent, render, screen } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import type { SyncPlan } from "../bindings";
import { SyncReview } from "./SyncReview";

const none = { added: [], removed: [], changed: [] };
const plan: SyncPlan = {
  from_commit: "a".repeat(40),
  to_commit: "b".repeat(40),
  platform_changed: false,
  widgets: { ...none, changed: ["status-page"] },
  dashboards: none,
  diagnostics: [],
  capabilities: {
    "status-page": { hosts: ["https://status.acme.dev"] },
    "other-widget": { hosts: ["https://old.acme.dev"] },
  },
  capability_hash: "sha256:y",
  capability_changes: {
    "status-page": { added: { hosts: ["https://status.acme.dev"] }, removed: {} },
  },
  needs_consent: true,
};

const show = (p: SyncPlan, onApply = vi.fn()) => {
  render(() => (
    <SyncReview plan={p} firstSync={false} busy={false} onApply={onApply} onDiscard={() => {}} onReview={() => {}} />
  ));
  return onApply;
};

describe("SyncReview", () => {
  it("lists only added capabilities per widget", () => {
    show(plan);
    const text = screen.getByTestId("capabilities").textContent!;
    expect(text).toContain("New permissions");
    expect(text).toContain("status-page");
    expect(text).toContain("https://status.acme.dev");
    expect(document.body.textContent).not.toContain("old.acme.dev");
    expect(document.body.textContent).not.toContain("other-widget");
  });

  it("shows removals under No longer needed", () => {
    show({
      ...plan,
      capability_changes: {
        ...plan.capability_changes,
        legacy: { added: {}, removed: { executables: ["kubectl"] } },
      },
    });
    const removed = screen.getByTestId("capabilities-removed");
    expect(removed.textContent).toContain("No longer needed");
    expect(removed.textContent).toContain("legacy");
    expect(removed.textContent).toContain("kubectl");
  });

  it("Apply, keep these blocked calls onApply(false)", () => {
    const onApply = show(plan);
    fireEvent.click(screen.getByText("Apply, keep these blocked"));
    expect(onApply).toHaveBeenCalledWith(false);
  });

  it("Approve and apply calls onApply(true)", () => {
    const onApply = show(plan);
    fireEvent.click(screen.getByText("Approve and apply"));
    expect(onApply).toHaveBeenCalledWith(true);
  });

  it("offers a plain Apply when nothing needs consent", () => {
    const onApply = show({ ...plan, capability_changes: {}, needs_consent: false });
    expect(screen.queryByTestId("capabilities")).toBeNull();
    expect(screen.queryByText("Approve and apply")).toBeNull();
    fireEvent.click(screen.getByText("Apply"));
    expect(onApply).toHaveBeenCalledWith(false);
  });
});
