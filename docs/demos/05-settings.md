# Demo — Slice 5: Settings (inputs, secrets, repo connection)

## Setup

```sh
rm -rf ~/Library/Application\ Support/io.pharos.desktop
rm -rf /tmp/platform && cp -r examples/platform-repo /tmp/platform
git -C /tmp/platform init -q -b main && git -C /tmp/platform add -A && git -C /tmp/platform commit -qm init
npm run dev
```

Connect `/tmp/platform` (branch `main`) and **Approve and apply** the review, as in
[02-sync.md](02-sync.md) steps 5–9.

## Setup from a widget

1. On Home, *My Pipelines* shows **Needs setup**: *Needs Your team · Needs secret GITLAB_TOKEN*.
   Click **Open Settings** → Settings opens with *Your team* focused.
2. Pick `payments-team` → "Saved". Under *Secrets*, type any token into *GITLAB_TOKEN* →
   **Save** → the row says **Set** and the field is gone. **← Back to dashboards** →
   *My Pipelines* now says **Not available yet**: *http sources arrive in a later version.*
   (The http executor is slice 6.)
3. Quit the app and run `npm run dev` again → Settings still shows the team and *Set*.
   In **Keychain Access**, search `io.pharos.desktop`: one item, account
   `acme-platform/GITLAB_TOKEN`. The token is not in `pharos.db`:
   `grep -c <your token> ~/Library/Application\ Support/io.pharos.desktop/pharos.db` → `0`.
4. **Remove** the secret → confirm in the dialog → back on Home, *My Pipelines* says
   *Needs setup — Needs secret GITLAB_TOKEN*. The Keychain item is gone.

## Fixing the repo connection

5. Settings → **Disconnect…** → leave *Also forget my details and secrets for ACME Developer
   Portal* unticked → **Disconnect** → onboarding shows with `/tmp/platform` and `main`
   pre-filled. Connect and approve → Settings still shows `payments-team`.
6. Set the token again, then **Disconnect…** with the box ticked → reconnect → *Your team* is
   empty, the secret is *Not set*, and the Keychain item is gone.

## When the repo changes under your settings

7. Set `payments-team`, then remove it from the catalog:
   `git -C /tmp/platform rm -q catalog/teams/payments-team.yaml && git -C /tmp/platform commit -qm drop`
   → **Sync now** → Settings shows *payments-team is no longer offered, pick another*, and
   *My Pipelines* needs your team again.

Undeclared names, the trimming of pasted values, keyring failures during disconnect and the
*No longer used* list are covered by `cargo test -p pharos-engine` and `npm test -w app`.
