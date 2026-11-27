# Data shaping gaps + a starter widget pack — design (slice 11)

Close the four gaps that keep platform teams from shipping useful widgets with
the current runtimes (`file`, `command`, `http`), then ship a starter pack of
widgets and dashboards in `examples/platform-repo/` that uses them.

Follow the usual repo workflow (CLAUDE.md, SESSIONS.md, session planner): TDD,
one branch per slice, bindings regenerated when Rust types change, a demo script
in `docs/demos/`, and every decision logged. Ids were renumbered on intake (S-017) to the next free ones: tasks **T-026–T-030**, decisions **D-041–D-044**.

## Suggested backlog

| Task | What | Size | Depends on |
|---|---|---|---|
| T-026 | Row selector (`source.rows`) | M | — |
| T-027 | `kv` view | S | — |
| T-028 | Refresh scheduler (`spec.refresh`) | M | — |
| T-029 | Markdown from a source | S | — |
| T-030 | Starter widget pack + dashboards + demo | M | T-026..T-029 |

T-026 to T-029 are independent and can be done in any order (or in parallel by
subagents). T-030 goes last. Treat the whole thing as one slice ("data shaping +
starter pack") placed before the command palette: it is **slice 11** in `docs/restart-plan.md` §5, and the palette moves to slice 12 (T-012 postponed).

---

## Constraints that apply to every task

- **Nothing executable crosses IPC.** The engine keeps resolving everything from
  the stored manifest. No new command takes a URL, path, executable, args or
  selector from the frontend.
- **Consent stays static.** None of these features adds a capability. `rows`,
  `refresh` and view changes must not change the derived capabilities, so they
  must not trigger re-consent. Add a test that proves it for `rows`.
- **Limits stay as they are** (D-032, D-034): 5 MB, 500 rows, existing timeouts.
  The 500-row cap applies **after** row selection.
- **`pharos-manifest` and `pharos-engine` stay Tauri-free.**
- **Lint must catch authoring mistakes** with the usual structured errors
  (file, path, message), each with a fixture test.

---

## T-026 — Row selector

**Problem.** `run::rows()` only produces one row per item when the JSON is an
array at the top level. Most real responses wrap the list in an object:
`kubectl -o json` → `{ "items": [...] }`, GitHub → `{ "workflow_runs": [...] }`,
most paging and alerting APIs → `{ "data": [...] }`. Today these become one
useless row, so they can't feed a table.

**Authoring shape** (YAML stays flat and readable):

```yaml
source:
  type: command
  executable: kubectl
  args: [get, pods, -n, "{{ inputs.namespace }}", -o, json]
  rows: items
```

`rows` is an optional field on **every** source type. Implementation detail is
your call (a field on each `Source` variant, or a wrapper with `#[serde(flatten)]`;
watch the interaction between `flatten`, internally tagged enums and
`deny_unknown_fields`). The YAML shape above is what matters.

**Semantics.**
- The value is a dot path with the **same syntax and semantics as column
  `field`** (`a.b`, numeric segments index into arrays: `contexts.0`). The
  frontend has this logic in `app/src/views/fields.ts`; the Rust side must match
  it, with shared test cases, so authors learn one syntax.
- Applied to the parsed JSON **before** `rows()`. If the selected value is an
  array, each item becomes a row. If it's an object or scalar, one row.
- `file` source: applied **per file document**, then rows from all files are
  concatenated. `_source` is still set on every row.
- `command` / `http`: applied to the whole parsed response.
- Path not found → a clear run error (`` `rows: items` matched nothing in the
  output ``), not an empty table. An empty table hides authoring mistakes.
- `rows` set but the output isn't JSON (command text output) → run error saying so.
- No wildcards, filters or expressions. Filtering belongs to slice 14
  (`filter`, team scoping). Note it in the docs so nobody adds it here.

**Implementation notes (approved S-017).**
- `rows: Option<String>` is a field on each `Source` variant (serde `flatten` is unreliable with the
  internally tagged, `deny_unknown_fields` enum), exposed via `Source::rows()`.
- The Rust selector mirrors `app/src/views/fields.ts` exactly (split on `.`, arrays indexed by
  `Number(key)`); one shared case table, `testdata/dot-paths.json`, is read by both `cargo test` and
  vitest. Lint rejects empty segments and whitespace, so `fields.ts` quirks (`""` → index 0) can't arise.
- Errors name the path: `` `rows: items` matched nothing in the output `` and
  `` `rows: items` needs JSON output, but the command printed text `` (http: "…but the response was not JSON").
- **Row cap for file sources too:** after selection every source type is capped at 500 rows with
  `truncated` set. Previously file sources capped the number of files (500) but not the rows inside
  them; the file cap stays. The frontend note becomes "Only the first 500 rows are shown."
- Logged as D-041.

**Lint.** Valid dot path syntax. `rows` is rejected on a widget without a source
(can't happen if it lives on the source, but keep the error if you put it elsewhere).

**Tests.** Manifest parsing + lint fixtures. Engine: one test per source type
(file per-document flattening, command, http with a mocked server), not-found
error, non-JSON error, truncation applied after selection, capabilities and
consent hash unchanged when only `rows` changes.

---

## T-027 — `kv` view

**Problem.** `kv` is in the schema but `dashboard.rs` reports it as "not
available". It's the natural view for identity and context cards (AWS caller
identity, current kube context).

**Behaviour.**
- Renders **one** row as label/value pairs.
- With `columns`: those fields, in that order, with their labels (dot paths
  work like in tables).
- Without `columns`: every top-level scalar field of the row, in key order;
  nested objects and arrays are shown as compact JSON (monospace), not expanded.
  `_source` is hidden unless listed explicitly.
- More than one row → render the first and a muted note (`+ N more rows — use a
  table view`). Zero rows → the existing empty state.
- Long values wrap; values that look like ARNs, ids or URLs use the monospace
  style already used in tables. URLs follow the same link rules as tables.
- Styling: follows DESIGN.md / D-036 tokens. Labels in `--muted`, values in
  `--fg`, rows separated by `--border-soft`. No new tokens.

**Implementation notes (approved S-017).** Tables use neither monospace nor clickable links today
(D-026), so for `kv`: values starting with `arn:`, `http://` or `https://`, or shaped like a UUID or
a long hex id (≥ 12 hex chars), render in `--mono` (the Machine Text Rule); URLs stay plain text.
A `kv` view without a source reports "a kv view without a data source". Logged as D-042.

**Engine.** Make `(Some(_), ViewType::Kv)` runnable in `dashboard.rs`. A `kv`
view without a source stays "not available" (there's nothing to show).

**Tests.** vitest for columns / no columns / multi-row note / empty / nested
value. Engine availability test updated.

---

## T-028 — Refresh scheduler

**Problem.** `spec.refresh` is parsed and present in the examples but nothing
uses it (open item on T-007 and T-008). Pipelines and on-call widgets look
dead without it.

**Design (frontend ticker, per restart-plan lesson 9: one scheduler, not one
per widget).**
- One ticker in the central store drives refresh for the **visible** dashboard
  only. It calls the existing `run_placement`. No new IPC.
- Each leaf placement refreshes when `now - lastRun >= refresh`.
- **Pause** when the document is hidden (`visibilitychange`), when the window
  isn't focused for more than a minute, while a sync is pending review, and in
  dashboard edit mode. On resume, refresh anything overdue once (not a burst
  per missed tick).
- **No overlap:** skip a placement whose previous run is still in flight.
- **Serialize and stagger.** A running command or http call holds the engine
  lock for up to its timeout (noted on T-008), so never fire more than one run
  at a time from the ticker, and add a small random jitter (±10%) so widgets
  with the same interval don't line up.
- **Back off on errors:** after a failed run, double the interval up to 10
  minutes; reset on success. NeedsSetup / NotAvailable placements are never
  scheduled.
- Widget header shows "updated 2m ago" (relative, updates with the ticker) and
  a manual refresh button if there isn't one yet. Manual refresh resets that
  placement's timer.
- `Sync now` keeps re-running everything (D-032) and resets all timers.

**Manifest / lint.** Parse durations as `<n>s|m|h`. Minimum **15s** (lint error
below it, with the minimum in the message). No `refresh` = no automatic refresh.

**Tests.** vitest with fake timers: interval respected, pause on hidden, resume
runs overdue once, no overlap, one run at a time, backoff and reset, edit mode
pauses. Lint fixtures for bad format and below-minimum.

---

## T-029 — Markdown from a source

**Problem.** `markdown` only renders static `content`. A platform team wants to
ship announcements, release notes or a runbook as a `.md` file in the repo (or
fetch one), and have it rendered.

**Behaviour.**
- `view.type: markdown` with a source renders the text of the **first row's
  `value` field**, or of the field named by a single `columns` entry if given.
- `content` and `source` are mutually exclusive (lint error if both).
- **file source:** `.md` and `.txt` files are read as text, producing one row
  `{ value: <text>, _source }` per file, instead of going through the YAML
  parser. Other extensions keep today's behaviour.
- **command source:** already produces `{ value }` for non-JSON stdout (D-034).
  Nothing to change.
- **http source:** today a non-JSON body is a `Parse` error. When the widget's
  view is `markdown`, accept `text/markdown` / `text/plain` bodies and produce a
  `{ value }` row. Keep the JSON-only behaviour for every other view, and keep
  sending `Accept: application/json` unless the view is markdown.
- Rendering reuses `app/src/views/markdown.tsx` (mdast, no raw HTML). Cap the
  rendered text at 256 KB with a truncation note.
- Engine availability: `(Some(_), ViewType::Markdown)` becomes runnable.

**Tests.** Engine: `.md` file read as text, http text body accepted only for
markdown, both-content-and-source lint error. vitest: renders from `value`,
from a named field, truncation note.

---

## T-030 — Starter widget pack

Replace and extend `examples/platform-repo/` so it demonstrates every runtime
and every gap above. It's what new users and the README walkthrough will see,
so it should look like something a real platform team would ship.
`pharos lint examples/platform-repo` must pass.

### Widgets

| Widget | Source → view | Shows off |
|---|---|---|
| `service-catalog` (exists) | file `catalog/services/*.yaml` → table | Add a `runbook` column and a `description` field to the service files |
| `team-directory` (new) | file `catalog/teams/*.yaml` → table | name, slack, on-call alias, lead. Extend the team files with those fields |
| `k8s-context` (rewrite) | command `kubectl config view --minify -o json`, `rows: contexts` → **kv** with columns `name`, `context.cluster`, `context.namespace`, `context.user` | row selector + kv |
| `aws-identity` (new) | command `aws sts get-caller-identity --output json` → **kv** (`Account`, `Arn`, `UserId`) | Command args already support `{{ inputs.* }}`; add an optional `profile` input passed as `--profile`, and make sure an empty optional input doesn't produce an empty argument. Expired SSO must show the CLI's stderr clearly (it already should, per D-034) |
| `my-merge-requests` (new) | http GitLab `/api/v4/merge_requests?scope=assigned_to_me&state=opened`, `PRIVATE-TOKEN: {{ secrets.GITLAB_TOKEN }}` → table (`title`, `references.full`, `author.username`, `web_url`) | top-level array, no selector needed |
| `my-pull-requests` (new) | command `gh search prs --author=@me --state=open --json title,url,repository` → table | GitHub alternative, vendor-neutral example |
| `my-pipelines` (exists) | http GitLab pipelines → table, `refresh: 1m` | refresh scheduler |
| `pods` (new) | command `kubectl get pods -n {{ inputs.namespace }} -o json`, `rows: items` → table (`metadata.name`, `status.phase`, `spec.nodeName`) | row selector on the canonical wrapped response; `namespace` is a required string input |
| `announcements` (new) | file `docs/announcements.md` → markdown | markdown from a source; `refresh: 5m` |
| `welcome` (exists) | static markdown | keep, tighten the copy |

Add `docs/announcements.md` to the example repo with two or three realistic
entries (a deprecation notice with a date, a new golden path, a maintenance window).

### Dashboards

- `home` (existing, re-laid out): `welcome`, `announcements`, `k8s-context`,
  `aws-identity`, `service-catalog`, `my-merge-requests`, `my-pipelines`. The
  identity cards (kv) sit together at the top right; the catalog gets the widest
  slot.
- `teams` (new): `team-directory` + `service-catalog`.
- `cluster` (new): `k8s-context` + `pods` (with a `namespace` placement input).
- Keep the `on-call-overview` composite working.

### Docs

- README walkthrough and widget authoring docs: document `rows`, `kv`,
  `refresh` (with the 15s minimum and the pause rules), and markdown from a
  source, each with a short YAML example.
- Include the tip that CLI query flags (`aws --query`, `gh --jq`) remain a valid
  alternative to `rows` when a reshape is needed.

### Demo

`docs/demos/11-data-shaping.md` (or the next free number), real app, both
themes: sync the example repo, approve consent, show kv cards, the pods table
via `rows: items`, announcements rendered from a file, a pipelines widget
refreshing on its own (with "updated … ago" moving), pause on window hide, and
a deliberate `rows` typo producing the clear error.

---

## Decisions to log (proposals; adjust ids to what's free)

- **D-041** Row selector: `source.rows`, same dot path syntax as columns,
  applied before row conversion (per document for file sources), not-found is
  an error, no filters (slice 14 owns filtering). Doesn't affect capabilities
  or consent.
- **D-042** `kv` view renders one row; columns optional; extra rows noted, not
  rendered.
- **D-043** Refresh: one frontend ticker over the visible dashboard via
  `run_placement`, minimum 15s, paused when hidden, unfocused, in edit mode or
  during sync review; serialized, jittered, error backoff to 10 min.
- **D-044** Markdown from a source: `.md`/`.txt` files read as text, http text
  bodies accepted only for markdown views, rendered through the existing mdast
  renderer with a 256 KB cap.

## Done when

- `cargo fmt --check`, `cargo clippy`, `cargo test` (all crates), `tsc`,
  `vitest`, bindings drift check: all green.
- `pharos lint examples/platform-repo` passes; new lint fixtures cover every
  new error.
- The demo has been clicked through in the real app on macOS (WebKit), light
  and dark.
- SESSIONS.md updated: tasks, decisions, and any deferred minors.
