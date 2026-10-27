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
   **Local IDP** with `/tmp/platform · main`.
6. Quit and run `npm run dev` again → the app opens on that screen, skipping onboarding.
   (`~/…` paths work too: they are expanded before git sees them.)
