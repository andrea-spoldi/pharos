# Demo — Slice 6: HTTP executor

*My Pipelines* calls an http API with the user's team and a keyring secret. A local file
server stands in for GitLab, so no real token is needed. The last part calls gitlab.com.

## Setup

```sh
rm -rf ~/Library/Application\ Support/io.pharos.desktop
rm -rf /tmp/platform && cp -r examples/platform-repo /tmp/platform
sed -i '' 's#https://gitlab.com#http://127.0.0.1:8765#' /tmp/platform/widgets/my-pipelines/widget.yaml
git -C /tmp/platform init -q -b main && git -C /tmp/platform add -A && git -C /tmp/platform commit -qm init

# A stand-in for GitLab: serves one canned pipelines list.
rm -rf /tmp/fake-gitlab && mkdir -p /tmp/fake-gitlab/api/v4/projects/acme/payments-team
printf '[{"id":101,"status":"success","ref":"main","web_url":"https://gitlab.acme/p/101"},{"id":100,"status":"failed","ref":"fix-login","web_url":"https://gitlab.acme/p/100"}]' \
  > /tmp/fake-gitlab/api/v4/projects/acme/payments-team/pipelines
python3 -m http.server 8765 --bind 127.0.0.1 --directory /tmp/fake-gitlab
```

In a second terminal, run `npm run dev`. Connect `/tmp/platform` (branch `main`).

## Calling an approved host

1. The review lists my-pipelines *Call* `http://127.0.0.1:8765`, with an *insecure-http*
   warning because the stand-in has no TLS. **Approve and apply**.
2. *My Pipelines* shows **Needs setup**. Click **Open Settings**, pick `payments-team` and
   save any token, for example `glpat-demo`, as *GITLAB_TOKEN*. Then click
   **← Back to dashboards**.
3. *My Pipelines* lists pipelines **101** (success, `main`) and **100** (failed, `fix-login`).
   The server terminal logs
   `GET /api/v4/projects/acme%2Fpayments-team/pipelines?per_page=10`. The team was
   filled in, and `acme%2F` from the dashboard was kept as written.
4. Stop the server (Ctrl-C) and click **Sync now**. *My Pipelines* says
   **Could not reach the service**. **Details** shows the host and the error, but never the
   token. Start the server again and click **Sync now**: the rows are back.

## Only what was approved

5. Point the widget at another host:
   `sed -i '' 's#127.0.0.1:8765#localhost:8765#' /tmp/platform/widgets/my-pipelines/widget.yaml && git -C /tmp/platform commit -qam host`
   Then click **Sync now**. The review asks again, now for *Call* `http://localhost:8765`.
   Click **Discard**. The dashboard still calls the approved `127.0.0.1` and keeps working.

## The real GitLab

6. Restore the URL and apply it:
   `sed -i '' 's#http://localhost:8765#https://gitlab.com#' /tmp/platform/widgets/my-pipelines/widget.yaml && git -C /tmp/platform commit -qam gitlab`
   Click **Sync now**, then **Approve and apply** *Call* `https://gitlab.com`.
7. With the made-up token, *My Pipelines* says **Access denied**: *The service refused the
   request.* **Details** shows `https://gitlab.com answered 401: {"message":"401 Unauthorized"}`.
   **Open Settings** goes to Settings to replace the token.

With a real token with `read_api` scope, set your own project in `dashboards/home.yaml`
(`project: "<group>%2F<project>"`) to see real pipelines.

Templating, URL encoding, declared-secret checks, redirects (never followed), error
redaction and the 401 path are covered by `cargo test -p pharos-engine --test http`.
