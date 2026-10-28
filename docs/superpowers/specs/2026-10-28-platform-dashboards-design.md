# Slice 4 — Platform dashboards (T-005) — design

Status: draft for approval · 2026-10-28 · session S-006

## Intent

The app is becoming a developer's **Monday-morning start page**: what's on my day, what I
have to do, what's broken, and what other teams are shipping. Platform dashboards are where
that page lives. The platform team decides *what* is on a dashboard and *where* (the
`kind: Dashboard` YAML). The app only owns the frame around it: switching dashboards, where
sync lives, and what a widget looks like while it loads, fails or can't run yet.

The user has no settled UX vision yet. This slice should therefore make **cheap, reversible
frame choices** and put the content first. It should not try to define the product's look.

Slice 4 "done when" (restart-plan §5): **the Home dashboard from the repo appears after sync
with zero user clicks.**

## Goals

1. Render platform dashboards (`kind: Dashboard`, read-only) as a 12-column grid placed from
   each placement's `x/y/w/h`.
2. A dashboard switcher.
3. One widget frame with a fixed set of states, designed once and used by every widget.
4. A safe markdown view, because the example Home dashboard includes a markdown widget
   (`welcome`).
5. The platform title in the header.

## Non-goals (this slice)

- User dashboards, Customize/fork, adding widgets (slice 8). Platform dashboards are
  read-only, with no edit affordances.
- Drag/resize (slice 10).
- Filling in inputs or secrets (slice 5). This slice only *shows* that setup is needed.
- Running command/http widgets (slices 6–7).
- Widget instances and `run_instance(id)`. Widgets keep running by name (D-021). Placement
  inputs don't affect file widgets, whose paths are literal (D-010), so instances can wait
  for slice 8, where user placements need them.
- Per-widget refresh, refresh timers, "Needs approval" state (slice 9).
- The flat widget list. It is removed; the Catalog screen (slice 8) replaces it.

## Anti-goals

- Anything that blocks the dashboard on the user: no extra click after sync, no modal.
- Rendering repo content as HTML (`innerHTML`) anywhere, markdown included.
- The frontend deciding what runs or parsing templates. The engine says what a placement
  needs, and the frontend only displays it.

## Screen

```
┌────────────────────────────────────────────────────────────────────────┐
│ ACME Developer Portal   repo · main · abc1234          [ Sync now ]    │  top bar
│ ───────────────────────────────────────────────────────────────────────│
│  Home   Services   …   More ▾                                          │  tabs
│ ┌──────────────────────────────────────┐ ┌───────────────────┐         │
│ │ Service Catalog                      │ │ K8s Context       │         │
│ │ table (scrolls inside the card)      │ │ Not available yet │         │
│ │                                      │ ├───────────────────┤         │
│ │                                      │ │ Welcome (md)      │         │
│ └──────────────────────────────────────┘ └───────────────────┘         │
│ ┌────────────────────────────────────────────────────────────────────┐ │
│ │ My Pipelines — Needs setup: your team                              │ │
│ └────────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────────┘
```

- **Top bar:** platform title (`cockpit.yaml` `metadata.title`, else `metadata.name`, else
  "Local IDP"), then the repo line as today, with **Sync now** on the right. The sync status,
  changelog and errors behave exactly as in slice 2 (D-019). The changelog stays a dismissible
  notice above the tabs.
- **Tabs:** one per platform dashboard, label `metadata.title` else `metadata.name`, ordered
  by name with `home` first. The first 5 are tabs and the rest go in a **More ▾** menu,
  which closes on selection, outside click and Escape. If there's a single dashboard, still
  show its tab, so the frame doesn't jump when a second one is added.
- **Grid:** 12 columns, fixed row height of 80px. A placement maps to
  `grid-column: x+1 / span w; grid-row: y+1 / span h`. A card's body scrolls internally, so
  content never grows the grid. Below 900px wide, cards stack in one column in reading
  order (sort by `y`, then `x`) with a capped height.
- **No dashboards:** before the first apply, Home keeps today's "Nothing from the platform
  repo is applied yet / Review the repo" notice. After an apply with zero dashboards, it shows a
  plain notice, "The platform repo defines no dashboards yet", with no tab bar (my call).
- **Pending review:** when a sync plan awaits review, the review takes over the main area,
  as today (D-019). Dashboards come back after apply or discard.

## Routing

Install `@solidjs/router` (already the planned stack) and use a **hash router**, so a reload
works under the Tauri asset protocol.

- `#/d/:name` shows that dashboard. `#/` and unknown names redirect (replace, not push) to
  the default: `home` if present, else the first by name.
- Onboarding and Home keep their current decision logic in `App`. The router only selects
  the dashboard inside Home.
- If the dashboard on screen disappears after a sync, go to the default.

## Widget states (the frame)

Every placement renders in one `WidgetFrame`: a card with a title header (widget
`metadata.title`, else name) and a body. The body shows **exactly one** state, checked in
this order:

| # | State | When | Shows |
|---|---|---|---|
| 1 | **Missing widget** | Placement names a widget not in the snapshot | Danger tone, names the widget. Lint already refuses this, so it's defensive only |
| 2 | **Needs setup** | The engine reports required user inputs or secrets for this placement | Setup tone: "Needs your *team*" / "Needs secret *GITLAB_TOKEN*". No action yet; slice 5 adds the link to the form |
| 3 | **Not available yet** | Source is command/http, or the view type has no renderer (`kv`, `raw`) | Muted, says which part is missing |
| 4 | **Loading** | First run in flight | Skeleton lines. A re-run after sync keeps the old data visible (no flash) |
| 5 | **Error** | Run failed | Title by `kind` (existing: *Access denied*, *Could not read the data*, *Could not run this widget*), message in a collapsed **Details** |
| 6 | **Empty** | Run returned 0 rows | "No data in `<reads>`" (repo-relative path from the engine) |
| 7 | **Data** | Rows | The view (table/list/card). The truncation note stays |
| — | **Static** | No source, `markdown` view | The markdown content |

The views stop rendering their own "No data." (today's `Frame` in `app/src/views/index.tsx`),
because the frame owns emptiness.

## Backend

- **`list_dashboards() -> Vec<DashboardView>`** (new command; the engine builds the views
  from `dashboard_templates()` and the widget snapshot). A `DashboardView` is the dashboard's
  metadata plus its placements. Each placement carries:
  - `widget` (name), `x/y/w/h`;
  - `manifest: Option<Widget>`, the widget's stored manifest, for title, view type, columns
    and markdown `content`. It's `None` when the widget is missing;
  - **`status`**, decided by the engine so the frontend never decides what runs (my call):
    `missing` | `needsSetup { inputs: Vec<String>, secrets: Vec<String> }` |
    `notAvailable { reason }` (command/http source, or a view type without a renderer, e.g.
    `kv` and `raw`) | `static` (no source) | `runnable`. It uses the same precedence as the
    state table. `needsSetup` lists the `user.*` keys and `secrets.*` names referenced by the
    placement's input values and by the widget's templated source fields. It's computed with
    `cockpit_manifest::template`, which lint already uses. In slice 4 nothing is configured,
    so everything referenced is required. Slice 5 subtracts what the user has set;
  - `reads: Option<String>`, for file sources: the source path resolved **relative to the
    repo root** (e.g. `catalog/services/*.yaml`, glob included), used by the Empty state.
  The frontend renders state 1–3 / static / runnable from `status` alone.
  `list_widgets` stays, because the Catalog screen in slice 8 needs it, but dashboards don't
  call it.
- **`get_platform() -> Option<Platform>`** (new command): the stored platform manifest of
  the last applied sync (`sources.platform`), for the title. `None` before the first sync.
- `run_widget(name)` is unchanged (D-021).
- No schema migration is needed; everything comes from the snapshot.
- Bindings are regenerated by `cargo test` and never hand-edited.

## Markdown view

Parse with a markdown-to-AST library (my call: `mdast-util-from-markdown`, i.e. micromark,
small and no HTML output), then render the AST as Solid elements. Supported: headings,
paragraphs, emphasis/strong, inline code, code blocks, lists, block quotes, thematic breaks.
**Raw HTML nodes render as literal text.** Links render as text followed by the URL in muted
style; making them clickable needs the opener plugin and is out of scope. Images aren't
rendered (alt text only), since the CSP blocks remote images. The view handles both static
`content` and, later, markdown from a source. For this slice: static content only.

## Constraints

- CLAUDE.md security rules: nothing executable crosses IPC. The frontend sends only a widget
  name (run) and nothing for dashboards. Strict CSP stays as is, with no inline-HTML escape
  hatches.
- `cockpit-manifest` / `cockpit-engine` stay Tauri-free. New types derive `specta` behind the
  existing feature (D-016).
- Plain CSS with the existing tokens in `app/src/styles.css`, no component library. Light
  and dark both work.

## Tests and demo

- **Engine (cargo test):** `list_dashboards` returns placements in file order with
  setup requirements from the placement inputs (`{{ user.team }}`) and from widget source
  templates (`{{ secrets.X }}`, `{{ user.X }}`). A missing widget gets empty setup.
  `get_platform` returns `None` before the first apply and the manifest after it.
- **Frontend (vitest):** default route goes to `home`, else the first dashboard. Unknown
  `:name` redirects. Zero dashboards shows the empty notice with no tabs. Tabs switch and the More menu appears after 5. Each frame state renders
  for its condition, in the order above. The markdown view escapes raw HTML. Update the
  Home/App tests that assert the old widget list.
- **Demo:** `docs/demos/04-dashboards.md`, run in the real app:
  1. Fresh install, connect, approve: Home appears with no further click.
  2. Add a second dashboard in the repo, sync: a new tab appears without review (no
     capability change).
  3. Reload on the second tab: it stays.
  4. A narrow window stacks the cards.
  5. Each visible state is named.

## Left to the builder

- Component and file names (suggested: `app/src/screens/Dashboards.tsx`,
  `app/src/components/WidgetFrame.tsx`, `app/src/views/markdown.tsx`). `WidgetData`
  folds into the frame.
- Exact copy, spacing and skeleton look, within the tokens.
- Whether `DashboardView` reuses `Metadata` or flattens it.
- The spike on branch `spike/dashboards` is throwaway reference: build from `main` and
  copy pieces only with tests.

## Follow-ups (backlog, not this slice)

- **Feed widget: multi-source news aggregator** (after T-007). Pulls from JIRA,
  GitHub/GitLab releases, pipelines, alerts and the external service catalog, and merges
  them into one timeline sorted by time, each item tagged with its origin. Events are
  never stored as YAML in the repo; git only configures the sources. Needs a design for
  widgets with several sources.
- **Decide: personal todo widget.** It's data the user writes, which conflicts with D-003
  (no personal widgets) and with read-only widgets.
- **T-016** (runs while a sync awaits review) is still open. Dashboards make it more visible,
  but the review currently replaces the dashboard, so it's not hit in this slice's flows.
