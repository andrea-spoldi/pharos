# Demo — Slice 7: Command executor

*Kubernetes Context* runs `kubectl config current-context`. It uses no shell, has a timeout,
and runs only the approved executable, found in the program search path from Settings.
A throwaway `KUBECONFIG` keeps your own `~/.kube/config` untouched.

## Setup

```sh
rm -rf ~/Library/Application\ Support/io.pharos.desktop
rm -rf /tmp/platform && cp -r examples/platform-repo /tmp/platform
git -C /tmp/platform init -q -b main && git -C /tmp/platform add -A && git -C /tmp/platform commit -qm init
rm -f /tmp/pharos-kubeconfig
KUBECONFIG=/tmp/pharos-kubeconfig npm run dev
```

The app inherits `KUBECONFIG` and passes it on to kubectl. Connect `/tmp/platform` (branch `main`).

## Running an approved program

1. The review lists k8s-context *Run* `kubectl`. **Approve and apply**.
2. *Kubernetes Context* says **The command failed** and shows kubectl's own message,
   `` `kubectl` exited with code 1: error: current-context is not set ``. You don't
   need to open Details to see it.
3. In a second terminal, give the throwaway config a context:

   ```sh
   export KUBECONFIG=/tmp/pharos-kubeconfig
   kubectl config set-context kind-demo --cluster=kind-demo && kubectl config use-context kind-demo
   ```

   Click **Sync now**. *Kubernetes Context* shows `kind-demo` in monospace (the raw view).

## Where programs are found

4. Open **Settings**. Under **Programs**, *Program search path* lists `/opt/homebrew/bin`,
   `/usr/local/bin`, `~/.local/bin`, `/usr/bin`, `/bin`, `/usr/sbin` and `/sbin`.
5. Replace the list with `/usr/bin` and click outside the field. It says **Saved**.
   Type `bin` instead: it says *must be an absolute folder or start with ~/* and keeps
   `/usr/bin`.
6. Go back to the dashboards. *Kubernetes Context* says **Program not found**,
   `` `kubectl` was not found in /usr/bin ``. Click **Open Settings**: the search path
   field has focus. Click **Restore defaults**, then go back: `kind-demo` is back.
   (Kubernetes Context lives on `home`; if it still shows the old error, click **Sync now**.)

## Only what was approved

7. Point the widget at another program:

   ```sh
   sed -i '' 's#executable: kubectl#executable: /bin/echo#' /tmp/platform/widgets/k8s-context/widget.yaml
   git -C /tmp/platform commit -qam "run echo instead"
   ```

   Click **Sync now**. The review asks again, because k8s-context now runs `/bin/echo`.
   Click **Discard**: *Kubernetes Context* still runs kubectl from the applied snapshot.
8. A relative path is refused up front:

   ```sh
   sed -i '' 's#executable: /bin/echo#executable: bin/kubectl#' /tmp/platform/widgets/k8s-context/widget.yaml
   git -C /tmp/platform commit -qam "relative path"
   ```

   Click **Sync now**. The review shows the lint error *must be a program name or an absolute
   path* and can't be applied. Discard it.

## What the tests cover

`cargo test -p pharos-engine command` covers these: an unapproved executable is refused
before anything runs; args reach the program verbatim (`$HOME; echo x` isn't expanded);
a timeout kills the whole process group; JSON stdout becomes rows; stdout over 5 MB fails
the run; and a missing program names the folders searched.
