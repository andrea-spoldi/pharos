# Demo — Slice 10: Drag + resize your dashboard

On your own dashboards you can now move and resize top-level widgets directly on the grid. The
rest of the grid reflows live as you drag, and the layout persists. Platform dashboards and the
narrow single-column layout stay fixed — nothing is draggable there.

## Setup

```sh
rm -rf ~/Library/Application\ Support/io.pharos.desktop
rm -rf /tmp/platform && cp -r examples/platform-repo /tmp/platform
git -C /tmp/platform init -q -b main && git -C /tmp/platform add -A && git -C /tmp/platform commit -qm init
npm run dev
```

Connect `/tmp/platform` (branch `main`) and **Approve and apply** the review, as in
[02-sync.md](02-sync.md) steps 5–9. **Home** opens first.

## Arrange two widgets side by side

1. Click **+ New dashboard**, type `Layout` and **Create**. It opens as a new tab in edit mode.
2. Click **Add widget** → *Welcome*, then **Add widget** → *Service Catalog*. The two cards
   stack down the left column.
3. The dashboard is non-empty now, so a **Done / Edit** toggle appears. Stay in edit mode
   (the toggle reads **Done**). Hover a card: a **grab** cursor appears over its header and a
   small corner grip fades in at its bottom-right.
4. Drag the *Service Catalog* card by its **header** up and to the right so it sits beside
   *Welcome*. While you drag, the card lifts and carries the accent, its neighbours step back,
   and the grid reflows live. Release: the two cards are side by side.

## Grow one to push the other down

5. Drag the *Welcome* card's **bottom-right grip** to make it wider and taller. As it grows into
   the *Service Catalog* card's space, that card is pushed down and out of the way in real time.
   The grip clamps a leaf to at least 1×1 (a group to 3×2), so a card never collapses to nothing.
6. Begin another drag, then press **Escape** before releasing: the card snaps back to where it
   was and nothing is saved.

## Persistence and the fixed cases

7. Click **Done**, quit the app, and run `npm run dev` again (keep the app data). Open the
   *Layout* tab: the widgets are exactly where you left them — the geometry survived the reload.
8. Click **Done** (if still in edit mode) to view the dashboard: the grab cursor and grips are
   gone and the header no longer starts a drag. Open the **Home** tab (a platform dashboard):
   it has no grips at all, in view or any mode.
9. Narrow the window below ~900px so the grid collapses to one column: the grips disappear and
   dragging the header does nothing — the handlers are inert, not just hidden. Widen it again to
   restore drag/resize.

The reflow resolver, the `set_layout` command, and the move/resize wiring and gating are covered
by `npm test -w app` and `cargo test`; the true pointer feel is what this demo checks in the app.
