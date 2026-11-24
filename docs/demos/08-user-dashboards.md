# Demo — Slice 8a: Your own dashboards

You can compose dashboards of your own from the platform's widget catalog. They live in the
app (not in the platform repo), and placement is automatic: there is no drag or resize.

## Setup

```sh
rm -rf ~/Library/Application\ Support/io.pharos.desktop
rm -rf /tmp/platform && cp -r examples/platform-repo /tmp/platform
git -C /tmp/platform init -q -b main && git -C /tmp/platform add -A && git -C /tmp/platform commit -qm init
npm run dev
```

Connect `/tmp/platform` (branch `main`) and **Approve and apply** the review, as in
[02-sync.md](02-sync.md) steps 5–9. **Home** opens first.

## Creating and filling a dashboard

1. Click **+ New dashboard**, type `My board` and click **Create**. The app opens it as a new
   tab after the platform ones. It shows *Add your first widget*, and its toolbar has
   **Add widget**, **Rename**, **Set as landing** and **Delete**. (Platform dashboards show
   only **+ New dashboard**.)
2. Click **Add widget** and pick *Welcome*. The dialog closes and the card appears.
3. Click **Add widget** again and pick *On-call overview* (marked *· group*). It renders as a
   titled sub-grid holding its two children, *Service Catalog* and *Kubernetes Context*.
   Each child runs like any other widget; *Kubernetes Context* shows **Not available yet**
   unless you did [07-command.md](07-command.md)'s setup.

## Landing and persistence

4. Click **Set as landing**. The button becomes **Clear landing** and a landing mark appears
   on the *My board* tab.
5. Quit the app and run `npm run dev` again (keep the app data). It opens on *My board* with
   both widgets in place. Click the *Home* tab: it is unchanged, with its four cards, and
   has no edit buttons beyond **+ New dashboard**.

The rest (rename, delete, removing a widget, errors, merge with platform dashboards) is
covered by `npm test -w app`; the store and landing rules by `cargo test -p pharos-engine`.
