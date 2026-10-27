# Demo — Slice 2: Source onboarding + sync

## Setup

```sh
# Start from a fresh install (dev builds keep their state here).
rm -rf ~/Library/Application\ Support/dev.cockpit.local-idp
# A platform repo to connect to.
rm -rf /tmp/platform && cp -r examples/platform-repo /tmp/platform
git -C /tmp/platform init -q -b main && git -C /tmp/platform add -A && git -C /tmp/platform commit -qm init
npm run dev
```

## Onboarding (part b)

1. The window opens on **Connect your platform repo** with `✓ git version …` and a form
   whose branch field says `main`. **Connect** is disabled until a URL is typed.
2. Enter `/tmp/nope` → **Connect** → a red **git could not clone the repository** notice
   with git's own message (`repository '/tmp/nope' does not exist`) and the SSH hint.
   The form is usable again.
3. Enter `ext::sh -c touch% /tmp/pwned` → **Connect** → the notice shows
   `transport 'ext' not allowed`, and `ls /tmp/pwned` reports no such file.
4. Enter `/tmp/platform`, branch `-x` → **That branch name isn't valid**.
5. Branch `main` → **Connect** → the button reads **Cloning…**, then the window shows
   **Local IDP** with `/tmp/platform · main` (`~/…` paths work too: they are expanded
   before git sees them).
6. Quit and run `npm run dev` again → onboarding is skipped.

## Consent and widget list (part c)

7. Right after connecting, and on every launch until something is applied, the app fetches and lints the repo and shows
   **Review the platform repo**: `Widgets: added k8s-context, my-pipelines, service-catalog, welcome`,
   `Dashboards: added home`, `Platform settings changed`, and
   **What these widgets can do on your machine**: k8s-context *Run* `kubectl`,
   my-pipelines *Call* `https://gitlab.com`, service-catalog *Read* `catalog/services`.
8. **Discard** → *Nothing from the platform repo is applied yet.* → **Review the repo**
   brings the same review back.
9. **Approve and apply** → the header gains the short commit and **Widgets** lists four
   cards, e.g. *Kubernetes Context* · `k8s-context` · raw view · command · kubectl.

## Sync now and changelog (part d)

Keep the app open; edit `/tmp/platform` from a terminal between steps.

10. **Sync now** with nothing changed → *Already up to date.* The widget list stays put.
11. Change a title without touching capabilities:
    `sed -i '' 's/title: Welcome/title: Hello/' /tmp/platform/widgets/welcome/widget.yaml && git -C /tmp/platform commit -qam retitle`
    → **Sync now** → no review; a **Synced to `<sha>`** changelog with
    `Widgets: changed welcome`, and the card now reads *Hello*.
12. Add a capability:
    `printf '  permissions:\n    hosts: ["https://status.acme.dev"]\n' >> /tmp/platform/widgets/my-pipelines/widget.yaml && git -C /tmp/platform commit -qam host`
    → **Sync now** → **Review changes** with my-pipelines *Call* `https://gitlab.com, https://status.acme.dev`
    (the full set for now; showing only the new host is slice 9)
    → **Discard** → back to the list, still on the previous commit.
    **Sync now** again → the same review → **Approve and apply** → changelog.
13. Break the repo:
    `sed -i '' 's#../../catalog/services/#../../../../etc#' /tmp/platform/widgets/service-catalog/widget.yaml && git -C /tmp/platform commit -qam break`
    → **Sync now** → **1 lint error: this version can't be applied** listing
    `widgets/service-catalog/widget.yaml spec.source.path`, with no apply button.
    **Discard** → the last good widgets are still listed.
14. Quit and relaunch → the app syncs on launch without blocking: the widget list shows
    immediately and the same lint review appears once the fetch is done.
