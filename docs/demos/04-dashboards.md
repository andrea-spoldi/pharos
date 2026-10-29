# Demo — Slice 4: Platform dashboards

## Setup

```sh
rm -rf ~/Library/Application\ Support/io.pharos.desktop
rm -rf /tmp/platform && cp -r examples/platform-repo /tmp/platform
git -C /tmp/platform init -q -b main && git -C /tmp/platform add -A && git -C /tmp/platform commit -qm init
npm run dev
```

## Home appears after sync

1. Connect `/tmp/platform` (branch `main`) and **Approve and apply** the review, as in
   [02-sync.md](02-sync.md) steps 5–9. **Home** appears with no further click: the header
   says *ACME Developer Portal*, a *Home* tab is selected, and the address ends in `#/d/home`.
   The four cards, placed as in `dashboards/home.yaml`:
   - *Service Catalog* (left, ⅔ wide): a brief skeleton, then the services table.
   - *Kubernetes Context* (top right): **Not available yet**, *command sources arrive in a later version.*
   - *Welcome* (right): the markdown heading *Welcome to ACME* and its text.
   - *My Pipelines* (full width): **Needs setup**, *Needs your team · Needs secret GITLAB_TOKEN*.

## Switching dashboards

2. Add a second dashboard and sync:
   `printf 'apiVersion: pharos.io/v1alpha1\nkind: Dashboard\nmetadata: { name: services, title: Services }\nspec:\n  widgets:\n    - { widget: service-catalog, x: 0, y: 0, w: 12, h: 5 }\n' > /tmp/platform/dashboards/services.yaml && git -C /tmp/platform add -A && git -C /tmp/platform commit -qm services`
   → **Sync now** → changelog *Dashboards: added services* with no review (capabilities
   unchanged), and a *Services* tab appears after *Home*.
3. Click **Services**: the table fills the width. Reload (⌘R): it stays on Services.
4. Narrow the window below 900px: cards stack in one column, each scrolling inside itself.

## The current dashboard goes away

5. While on Services:
   `git -C /tmp/platform rm -q dashboards/services.yaml && git -C /tmp/platform commit -qm drop`
   → **Sync now** → the *Services* tab disappears and the view falls back to Home.

The other states (missing widget, error, empty, the More ▾ menu past five dashboards) are
covered by `npm test -w app`; placement statuses by `cargo test -p pharos-engine`.
