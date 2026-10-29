# Demo — Slice 3: File executor + views

## Setup

```sh
rm -rf ~/Library/Application\ Support/io.pharos.desktop
rm -rf /tmp/platform /tmp/outside.yaml && cp -r examples/platform-repo /tmp/platform
git -C /tmp/platform init -q -b main && git -C /tmp/platform add -A && git -C /tmp/platform commit -qm init
npm run dev
```

Connect `/tmp/platform` (branch `main`) and **Approve and apply** the review, as in
[02-sync.md](02-sync.md) steps 5–9.

## Running a file widget

1. The *Service Catalog* card shows a table with the columns **Service · Owner · Tier ·
   Repo · Source** and two rows, sorted by file:
   `notification-service · platform-team · 2 · gitlab.com/example/notification-service · catalog/services/notification-service.yaml`
   and `payment-service · payments-team · 1 · …`.
   The other cards (command, http, static) show no data yet.
2. Add a service and sync:
   `sed 's/payment-service/ledger-service/; s/payments-team/finance-team/' /tmp/platform/catalog/services/payment-service.yaml > /tmp/platform/catalog/services/ledger-service.yaml && git -C /tmp/platform add -A && git -C /tmp/platform commit -qm ledger`
   → **Sync now** → changelog *Synced to `<sha>`* with no review (capabilities unchanged), and
   the table shows three rows, *ledger-service* first.
3. Switch the view to cards:
   `sed -i '' 's/type: table/type: card/' /tmp/platform/widgets/service-catalog/widget.yaml && git -C /tmp/platform commit -qam cards`
   → **Sync now** → one card per service titled with its name, listing Owner, Tier, Repo
   and Source. Use `type: list` the same way → one line per service:
   **ledger-service** · Owner: finance-team · Tier: 1 · …

## Runs use the applied snapshot

4. Widen the source without applying it:
   `sed -i '' 's#"\*.yaml"#"*"#' /tmp/platform/widgets/service-catalog/widget.yaml && echo hi > /tmp/platform/catalog/services/notes.txt && git -C /tmp/platform add -A && git -C /tmp/platform commit -qm widen`
   → **Sync now** → the review lists `changed service-catalog`. **Discard** → the widget still
   shows the three services. `notes.txt` is not read because the applied manifest still says `*.yaml`.
   Undo: `git -C /tmp/platform reset -q --hard HEAD~1`.

## Confinement holds at run time

5. Link a catalog file outside the repo. It is catalog-shaped, so `pharos lint` accepts it:
   `printf 'apiVersion: v1\nkind: Service\nmetadata: { name: secret }\n' > /tmp/outside.yaml && ln -s /tmp/outside.yaml /tmp/platform/catalog/services/evil.yaml && git -C /tmp/platform add -A && git -C /tmp/platform commit -qm evil`
   → **Sync now** → the *Service Catalog* card shows a red **Access denied** notice:
   `` `catalog/services/evil.yaml`: path leaves the repository root through a symlink ``. No `secret` row appears.
   Undo: `git -C /tmp/platform rm -q catalog/services/evil.yaml && git -C /tmp/platform commit -qm undo`
   → **Sync now** → the table is back.

Unreadable files, the 500-file cap and unsupported views are covered by `vitest` and
`cargo test -p pharos-engine`.
