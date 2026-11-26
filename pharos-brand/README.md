# Pharos — brand marks (The Beam)

The mark: a lamp (source) with two light-shafts fanning upward; the gap between
them reads as the tower. Signal Blue (#2f5bd3 light / #6b8cff dark). No glow in the
mark itself — the glow is a UI treatment (the `--beam` token), not the logo.

## Vector masters
- `pharos-mark.svg`  — the mark alone, `fill: currentColor` (use in UI, wordmark lockups).
- `favicon.svg`      — same mark, auto-adapts to light/dark tab bars.
- `pharos-icon.svg`  — the app-icon tile (Signal Blue + white mark), 1024 artboard.

## App icons (Tauri)
`tauri.conf.json` expects: `icons/32x32.png`, `128x128.png`, `128x128@2x.png`,
`icon.icns`, `icon.png`. The PNGs are in `icons/`. **The authoritative way to
generate the full cross-platform set (including `icon.icns`, which isn't produced
here) is Tauri's own generator from the 1024 master:**

    cp icon-source.png <repo>/src-tauri/icons/icon-source.png
    cd <repo> && npm run tauri icon src-tauri/icons/icon-source.png

That regenerates 32x32.png, 128x128.png, 128x128@2x.png, icon.icns, icon.ico and
the Windows Store logos, overwriting the placeholders. If you'd rather not run it,
the PNGs here drop in directly — only `icon.icns` must still come from `tauri icon`.

## Favicon (frontend)
Copy `favicon.svg` into `app/` (e.g. `app/public/` or alongside `index.html`) and
add to `app/index.html` `<head>`:

    <link rel="icon" href="/favicon.svg" type="image/svg+xml">

Strict CSP already allows the app's own assets, so no CSP change is needed.

## Files
- `icon-source.png`     1024 app-icon master (feed to `tauri icon`)
- `icons/*.png`         ready-to-drop Tauri PNGs (+ `icon.ico` for Windows)
- `mark-16/32/48.png`   the mark alone (accent on transparent), small-size checks
- `contact-sheet.png`   overview at tile + small sizes, light & dark
