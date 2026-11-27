# T-023 — Report a widget's new permissions to the platform team

Date: 2026-11-27 · Session: S-015 · Backlog: T-023 · Builds on slice 9 (D-038)

## Intent
A widget blocked in **Needs approval** asks for something the user doesn't expect or won't allow.
Today their only choices are to approve it or leave it blocked. T-023 gives them a third: tell the
platform team, with the facts already written down, in the team's own tracker. Pharos never
posts anything itself: the user reviews and sends.

## Goals
- A **Report to platform team** action on the Needs approval card (leaf widgets only).
- It opens a pre-filled new issue (GitHub, GitLab) or email at an address the platform declares
  in `pharos.yaml`.
- No address declared → the same text is copied to the clipboard.
- The widget stays blocked; nothing is recorded; later syncs stay quiet (D-038 unchanged).

## Non-goals
- Pharos creating issues through an API, storing forge tokens, or tracking report status.
- Trackers beyond GitHub, GitLab and email (each is a small addition later).
- Clickable links anywhere else (markdown links stay text, D-026).

## Anti-goals
- The frontend supplying or seeing a URL to open. It sends only the widget name.
- Reporting anything the user wasn't shown: the report lists exactly the widget's pending
  capabilities (seen − approved), which is what the card displays.

## Approaches considered
- **A. Named service (chosen by user):** `spec.support: { type: github | gitlab | email, … }`;
  Pharos knows each one's prefill format. Lint can validate it; self-hosted GitLab works.
- **B. URL template** with `{{ report.title }}` / `{{ report.body }}`: any tracker, but lint can't
  check that the tracker accepts it and platform teams must get the template right.

## Design
**Contract (`pharos.yaml`, optional):**
```yaml
spec:
  support:
    type: github            # github | gitlab | email
    repo: https://github.com/acme/platform      # github / gitlab: the repo's web URL
  # or: { type: email, address: platform-team@acme.dev }
```
A tagged enum on `type`, unknown fields denied. Lint: `repo` is a literal `https` URL with a host
and no query, fragment or template; `address` is a literal address with one `@` and no `?`, `&`
or whitespace.

**Links** (all values percent-encoded):
- GitHub: `<repo>/issues/new?title=…&body=…`
- GitLab: `<repo>/-/issues/new?issue[title]=…&issue[description]=…`
- Email: `mailto:<address>?subject=…&body=…`

**Text** (exact copy):
- Title: `Pharos: widget "<name>" asks for new permissions`
- Body: one line naming the widget and the platform repo commit (short SHA) it was synced at;
  the pending capabilities as `Call …` / `Run …` / `Read …` lines; then
  `Why I haven't approved it:` followed by an empty line for the user.

**Engine (Tauri-free):** `Engine::report_widget(name) -> Report { title, body, link: Option<String> }`
built from the stored snapshot: `UnknownWidget` if absent, `NotALeaf` for a container, and a new
`NothingToReport` error when the widget has no pending capabilities. `link` is `None` when the
platform declares no `support`.

**Tauri:** add `tauri-plugin-opener`, used **from Rust only**. No `opener:` permission is added to
the frontend's capabilities, so the page can't open URLs. Command `report_widget(name)` opens
`link` in the default browser or mail client and returns `{ opened: bool, text: String }`, where
`text` is title + body for the clipboard fallback.

**UI (card):** **Report to platform team** sits next to **Approve**.
- Opened → status line *"Opened in your browser — send it from there."*
- No link → `navigator.clipboard.writeText(text)` → *"Copied — paste it to your platform team."*
- If copying fails, the text is shown in a read-only box to select by hand.
- Errors show inline (`role="alert"`). The widget stays blocked either way.

**Decisions:** D-039 records the support contract and opener-from-Rust-only. It narrows D-026:
links are still not clickable in content.

## Left to the builder
Error copy for the new error kind, CSS, test split, how the commit and repo appear in the body,
and the `examples/platform-repo` support entry for the demo (`docs/demos/09b-report-widget.md`).
