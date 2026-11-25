import { fireEvent, render, screen } from "@solidjs/testing-library";
import { createSignal } from "solid-js";
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

  it("traps Tab inside the dialog, wrapping at both ends", () => {
    render(() => (
      <Dialog
        title="Pick"
        open={true}
        onClose={() => {}}
        actions={
          <>
            <button type="button">Cancel</button>
            <button type="button">OK</button>
          </>
        }
      >
        <p>Body</p>
      </Dialog>
    ));
    const [cancel, ok] = screen.getAllByRole("button");
    // Tab off the last control wraps to the first.
    ok.focus();
    fireEvent.keyDown(document, { key: "Tab" });
    expect(document.activeElement).toBe(cancel);
    // Shift+Tab off the first wraps to the last.
    cancel.focus();
    fireEvent.keyDown(document, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(ok);
  });

  it("returns focus to whatever opened it when it closes", () => {
    const [open, setOpen] = createSignal(false);
    render(() => (
      <>
        <button type="button">Opener</button>
        <Dialog title="Pick" open={open()} onClose={() => setOpen(false)} actions={<button type="button">OK</button>}>
          <p>Body</p>
        </Dialog>
      </>
    ));
    const opener = screen.getByRole("button", { name: "Opener" });
    opener.focus();
    setOpen(true);
    expect(document.activeElement).toBe(screen.getByRole("dialog"));
    setOpen(false);
    expect(document.activeElement).toBe(opener);
  });
});
