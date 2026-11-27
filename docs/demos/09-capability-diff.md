# Demo — Slice 9: Capability diff and per-widget consent

When a sync makes a widget ask for more than you approved, the review lists only what is new, and you can
keep that widget blocked and approve it later from its own card. Removals never need consent.

## Setup

```sh
rm -rf ~/Library/Application\ Support/io.pharos.desktop
rm -rf /tmp/platform && cp -r examples/platform-repo /tmp/platform
git -C /tmp/platform init -q -b main && git -C /tmp/platform add -A && git -C /tmp/platform commit -qm init
npm run dev
```

Connect `/tmp/platform` (branch `main`) and **Approve and apply** the first review, as in
[02-sync.md](02-sync.md) steps 5-9. The widget used below is *Service Catalog*
(`widgets/service-catalog/widget.yaml`, a file widget): it runs offline and needs no setup.

## Add a host: the review shows only what is new

1. In `/tmp/platform/widgets/service-catalog/widget.yaml`, add this line under `spec:` (for example
   right above `refresh: 5m`) and commit it:

   ```yaml
     permissions: { hosts: ["https://status.acme.dev"] }
   ```

2. In the app click **Sync now**. The review opens and lists only the new host for Service Catalog;
   what you already approved (its catalog path) is not repeated.

## Keep it blocked

3. Click **Apply, keep these blocked**. The sync applies: other widgets in the change update as usual,
   and the Service Catalog card shows **Needs approval**, "This widget now wants to:" and the new host.
   It does not run.
4. Click **Sync now** again. No review opens (the change was already seen) and the card keeps its
   **Needs approval** state.

## Approve from the card

5. Click **Approve** on the card. No confirmation dialog: the widget reloads and runs, showing the
   services table again.

## Removals are silent

6. In `/tmp/platform/widgets/service-catalog/widget.yaml`, delete the `permissions:` line you added and
   commit. **Sync now** applies it without asking; the changelog says **Permissions removed:
   service-catalog**, and the widget keeps running.

## Clean up

```sh
rm -rf /tmp/platform
```

The diff, the held plan, `approve_widget` and the card's behaviour (including a group whose members are
all blocked showing each member's own card) are covered by `cargo test` and `npm test -w app`. The run in
the real app is for you to do.
