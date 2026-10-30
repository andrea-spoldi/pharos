import { fireEvent, render, screen } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import { Dialog } from "./Dialog";

describe("Dialog", () => {
  it("renders nothing when closed", () => {
    render(() => (
      <Dialog title="Remove it?" open={false} onClose={() => {}} actions={<button type="button">OK</button>}>
        <p>Body</p>
      </Dialog>
    ));
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("is a labelled modal that closes on Escape", () => {
    const onClose = vi.fn();
    render(() => (
      <Dialog title="Remove it?" open={true} onClose={onClose} actions={<button type="button">OK</button>}>
        <p>Body</p>
      </Dialog>
    ));
    const dialog = screen.getByRole("dialog", { name: "Remove it?" });
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    // The dialog itself, not its first (often destructive) button.
    expect(document.activeElement).toBe(dialog);
    fireEvent.keyDown(document, { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
  });
});
