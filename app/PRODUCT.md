# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

**Primary: developers** at a company whose platform engineering team distributes Pharos.
They open it on their own laptop (macOS or Linux) to see their team's services, pipelines,
cluster context and runbooks without going to a hosted portal. They connect once to the
platform repo, approve what the widgets may do on their machine, fill in personal inputs
(such as their team) and secrets (such as a GitLab token), and then mostly glance at
dashboards and press **Sync now**.

**Secondary: platform engineers** who write the platform repo (widgets, dashboards,
catalog YAML) and lint it in CI with `pharos lint`. They use the app to preview what
developers will see. When the two audiences pull in different directions, the developer
wins.

## Product Purpose

Pharos is a desktop "local IDP". The app clones the platform team's git repo, validates
it, shows what changed, asks for consent to new capabilities, and renders the platform's
dashboards. Widgets run locally with the developer's own credentials. Success means a
developer goes from a fresh install to a working Home dashboard in a few clicks, and later
platform updates arrive without breaking anything or asking for trust they don't need.

## Positioning

There is no server. Git is both the control plane and the data plane, and widgets run on
the developer's machine with their kubectl context, keyring secrets and local CLIs. Trust
is explicit and minimal: capabilities (hosts, executables, repo paths) are derived from
each widget's `spec.source`, approved once per capability set, asked again only when the
set changes, and enforced again at execution time. Widgets are declarative YAML only; there
is no custom widget code.

## Operating Context

- Tauri 2 desktop app (SolidJS webview) on macOS and Linux. Windows is out of scope for v1.
- Uses the system `git` with the user's SSH agent and credential helpers. The app never
  prompts for git credentials, and clone errors are shown verbatim with hints.
- Flow: onboarding (repo URL or local path plus branch) → review (changelog plus capability
  consent, or lint errors that block apply) → dashboards → settings (inputs, keyring
  secrets, repo connection).
- Sync happens on launch, without blocking, and on demand. There is no background timer.
- Platform repo contract: `pharos.yaml` (kind Platform), `widgets/*/widget.yaml`,
  `dashboards/*.yaml` (12-column grid), `catalog/**/*.yaml` (Kubernetes-style).

## Capabilities and Constraints

- Done: onboarding, sync with review and consent, file executor, table/list/card/raw/
  markdown views, read-only platform dashboards with shared widget states (empty, loading,
  error, needs setup), and settings.
- Planned: http and command executors, user dashboards with a Customize fork, a
  capability diff on re-sync, drag/resize layout, a ⌘K command palette, pre-configured
  builds, environments and team scoping.
- Not in v1: WASM, personal widgets, multiple platform repos, background sync, Windows.
- Terminology: *platform repo*, *widget*, *dashboard*, *catalog*, *capabilities*,
  *Sync now*, *Approve and apply*, *Needs approval*, *Setup required*.
- No browser `prompt()` or `alert()`; dialogs are always in-app. Strict CSP means the
  webview loads only the app's own scripts and styles, so there are no remote fonts or
  CDNs.
- Plain CSS with design tokens (light and dark). No component library.

## Brand Commitments

The app frame is **Pharos**. The platform's own identity (for example `metadata.title`
"ACME Developer Portal" in `pharos.yaml`) appears as content inside that frame and does
not replace it. Pre-configured builds may later override the app name and icon (slice 12).

## Evidence on Hand

- `examples/platform-repo/`: a complete sample platform with 4 widgets, a Home dashboard,
  and services and teams catalog data. It is the demo and test fixture.
- `docs/demos/*.md`: acceptance scripts for each slice. `README.md` §5 is a full
  walkthrough.
- There are no real customers, testimonials or usage metrics. Don't invent any.

## Product Principles

1. **Trust is visible.** Show what can run, call or read on the developer's machine, and
   ask again only when that changes.
2. **Never apply a broken or unapproved state.** Lint errors and missing consent block
   changes, and the last good state stays usable.
3. **Show what you have now.** Render the last snapshot right away and let syncs catch up
   without blocking.
4. **The platform curates and the user personalises.** Platform updates never wipe user
   inputs, secrets or forks.
5. **Errors explain the fix.** Give git's own message, the file and field, and the next
   step.
