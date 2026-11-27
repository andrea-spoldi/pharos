# Demo — T-023: Report a widget to the platform team

A widget stuck on **Needs approval** can be reported to the platform team from its card. Reporting
changes nothing: the widget stays blocked and nothing is stored.

## Setup

As in [09-capability-diff.md](09-capability-diff.md): reset app data, copy `examples/platform-repo` to
`/tmp/platform`, init and commit it, `npm run dev`, connect `/tmp/platform` (branch `main`) and
**Approve and apply** the first review. The example repo's `pharos.yaml` already has
`spec.support: { type: github, repo: https://github.com/acme/platform }`.

## Block Service Catalog

1. In `/tmp/platform/widgets/service-catalog/widget.yaml`, add this line under `spec:` (for example right
   above `refresh: 5m`) and commit it:

   ```yaml
     permissions: { hosts: ["https://status.acme.dev"] }
   ```

2. Click **Sync now**, then **Apply, keep these blocked**. The Service Catalog card shows **Needs approval**.

## Report with a link

3. Click **Report to platform team**. Your browser opens `github.com/acme/platform/issues/new` with the
   title and body pre-filled, listing the pending host (the repo need not exist to see the pre-fill). The
   card says "Opened in your browser — send it from there." and stays on **Needs approval**.

## Report without a link

4. Remove the `support:` line from `/tmp/platform/pharos.yaml`, commit, and **Sync now** (the card stays
   blocked). Click **Report to platform team** again. Nothing opens; the card says "Copied — paste it to
   your platform team." and the clipboard holds the report text.

## Clean up

Revert both edits (`git -C /tmp/platform checkout` or just remove the directory):

```sh
rm -rf /tmp/platform
```

The command, the report text and the card states (including the clipboard-failure fallback and inline
errors) are covered by `cargo test` and `npm test -w app`. The run in the real app is for you to do.
