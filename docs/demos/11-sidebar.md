# Demo — Slice 11: Sidebar navigation

Dashboards now live in a sidebar (open or rail) instead of a tab strip. Below 900px the sidebar gives way
to a compact top bar. Run on macOS (WebKit), once in light and once in dark (Settings, Appearance).

## Setup

```sh
rm -rf ~/Library/Application\ Support/io.pharos.desktop
rm -rf /tmp/platform && cp -r examples/platform-repo /tmp/platform
git -C /tmp/platform init -q -b main && git -C /tmp/platform add -A && git -C /tmp/platform commit -qm init
npm run dev
```

Connect `/tmp/platform` (branch `main`) and **Approve and apply** the first review, as in
[02-sync.md](02-sync.md) steps 5-9.

## Layout and switching

1. A fresh install opens at 1360x860 with the sidebar open (240px).
2. Click dashboards in the sidebar to switch. The current one has the accent wash and bar, and the landing
   dashboard shows its star.

## Your dashboards

3. Click **+ New dashboard**, give it a title. It appears under *Mine* and opens.
4. Open a platform dashboard and click **Customize**. The fork appears under *Mine*.

## Rail

5. Collapse with the sidebar button, then expand with Cmd+\ . Collapse again, quit and restart the app:
   the rail is kept.
6. Keyboard only: Tab through the sidebar. In rail mode each item shows its tooltip on focus.

## Sync

7. Click **Sync now** in the sidebar footer. The status updates. Make a capability change in
   `/tmp/platform` and commit it; **Sync now** still shows its review in the main area.

## Narrow window and layout

8. Shrink the window below 900px: the sidebar is replaced by a top bar with the dashboard menu.
9. Back at 1360 with the sidebar open, drag and resize a widget in edit mode. It still works.

## Clean up

```sh
rm -rf /tmp/platform
```

The sidebar, rail persistence, shortcut, top bar and status region are covered by `npm test -w app`.
The run in the real app is for you to do.
