---
name: Pharos
description: A calm desktop frame with a tinted page, white widgets that lift toward one lighthouse beam, and a single blue signal where attention belongs.
colors:
  signal-blue: "#2f5bd3"
  signal-ink: "#ffffff"
  accent-wash: "rgba(47, 91, 211, 0.09)"
  accent-weak: "rgba(47, 91, 211, 0.24)"
  page: "#eceff4"
  open-white: "#ffffff"
  mist-surface: "#f3f6fa"
  ink: "#1a1a1a"
  slate-muted: "#5f6672"
  fog-line: "#dce1ea"
  fog-soft: "#e8ebf1"
  buoy-green: "#1e7a46"
  flare-red: "#b42318"
  flare-wash: "#fdf1f0"
  flare-ink: "#ffffff"
  signal-blue-dark: "#6b8cff"
  signal-ink-dark: "#0d1020"
  accent-wash-dark: "rgba(107, 140, 255, 0.15)"
  accent-weak-dark: "rgba(107, 140, 255, 0.3)"
  page-dark: "#0e1016"
  deep-slate-surface: "#171a21"
  mist-surface-dark: "#1e222b"
  moon-gray: "#e6e6e6"
  slate-muted-dark: "#9aa1ad"
  fog-line-dark: "#262b35"
  fog-soft-dark: "#20242d"
  buoy-green-dark: "#4cc38a"
  flare-red-dark: "#ff8a7f"
  flare-wash-dark: "#2c1a1a"
  flare-ink-dark: "#1a0d0c"
typography:
  headline:
    fontFamily: "system-ui, -apple-system, sans-serif"
    fontSize: "1.5rem"
    fontWeight: 700
    lineHeight: 1.45
  page-title:
    fontFamily: "system-ui, -apple-system, sans-serif"
    fontSize: "1.35rem"
    fontWeight: 650
    lineHeight: 1.45
    letterSpacing: "-0.015em"
  title:
    fontFamily: "system-ui, -apple-system, sans-serif"
    fontSize: "1.15rem"
    fontWeight: 700
    lineHeight: 1.45
  title-small:
    fontFamily: "system-ui, -apple-system, sans-serif"
    fontSize: "1rem"
    fontWeight: 700
    lineHeight: 1.45
  body:
    fontFamily: "system-ui, -apple-system, sans-serif"
    fontSize: "1rem"
    fontWeight: 400
    lineHeight: 1.45
  body-small:
    fontFamily: "system-ui, -apple-system, sans-serif"
    fontSize: "0.9rem"
    fontWeight: 400
    lineHeight: 1.45
  label:
    fontFamily: "system-ui, -apple-system, sans-serif"
    fontSize: "1rem"
    fontWeight: 500
    lineHeight: 1.45
  nav-heading:
    fontFamily: "system-ui, -apple-system, sans-serif"
    fontSize: "0.72rem"
    fontWeight: 600
    letterSpacing: "0.06em"
  mono:
    fontFamily: "ui-monospace, \"SF Mono\", Menlo, monospace"
    fontSize: "0.85rem"
    fontWeight: 400
  mono-input:
    fontFamily: "ui-monospace, \"SF Mono\", Menlo, monospace"
    fontSize: "0.9rem"
    fontWeight: 400
rounded:
  base: "6px"
  inner: "4px"
spacing:
  space-1: "0.25rem"
  space-2: "0.5rem"
  space-3: "1rem"
  space-4: "2rem"
components:
  button-default:
    backgroundColor: "{colors.open-white}"
    textColor: "{colors.ink}"
    rounded: "{rounded.base}"
    padding: "0.5rem 1rem"
  button-primary:
    backgroundColor: "{colors.signal-blue}"
    textColor: "{colors.signal-ink}"
    rounded: "{rounded.base}"
    padding: "0.5rem 1rem"
  button-danger:
    backgroundColor: "{colors.flare-red}"
    textColor: "{colors.flare-ink}"
    rounded: "{rounded.base}"
    padding: "0.5rem 1rem"
  button-link:
    textColor: "{colors.signal-blue}"
    padding: "0"
  sidebar:
    backgroundColor: "{colors.open-white}"
    width: "240px"
  sidebar-rail:
    backgroundColor: "{colors.open-white}"
    width: "56px"
  nav-item:
    textColor: "{colors.slate-muted}"
    rounded: "{rounded.base}"
    padding: "6px 0.5rem"
  nav-item-hover:
    backgroundColor: "{colors.mist-surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.base}"
    padding: "6px 0.5rem"
  nav-item-current:
    backgroundColor: "{colors.accent-wash}"
    textColor: "{colors.ink}"
    rounded: "{rounded.base}"
    padding: "6px 0.5rem"
  nav-monogram:
    backgroundColor: "{colors.mist-surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.base}"
    size: "22px"
  narrow-bar:
    backgroundColor: "{colors.open-white}"
    padding: "0.5rem 1rem"
  segmented-option:
    textColor: "{colors.slate-muted}"
    rounded: "{rounded.inner}"
    padding: "0.25rem 1rem"
  segmented-option-selected:
    backgroundColor: "{colors.accent-wash}"
    textColor: "{colors.ink}"
    rounded: "{rounded.inner}"
    padding: "0.25rem 1rem"
  input:
    backgroundColor: "{colors.open-white}"
    textColor: "{colors.ink}"
    typography: "{typography.mono-input}"
    rounded: "{rounded.base}"
    padding: "0.5rem"
  card:
    backgroundColor: "{colors.open-white}"
    rounded: "{rounded.base}"
    padding: "1rem"
  widget-frame:
    backgroundColor: "{colors.open-white}"
    rounded: "{rounded.base}"
  notice:
    backgroundColor: "{colors.open-white}"
    rounded: "{rounded.base}"
    padding: "1rem"
  notice-danger:
    backgroundColor: "{colors.flare-wash}"
    rounded: "{rounded.base}"
    padding: "1rem"
  dialog:
    backgroundColor: "{colors.open-white}"
    rounded: "{rounded.base}"
    padding: "1rem 2rem"
---

# Design System: Pharos

## Overview

**Creative North Star: "The Harbour Light"**

Pharos is named after a lighthouse, and its interface works like one. The page is a cool
tinted neutral, like water at dusk; the widgets are crisp white and lift off it toward a
single soft beam at the top of the frame. One blue is the lamp, kept for the places where
the developer's attention belongs: the current dashboard in the sidebar, the one primary action, and
the prompt that says a widget needs setup. Everything else is quiet — system type, pale
tonal surfaces, thin fog-colored borders. The platform team's own content and data fill the
frame; Pharos itself stays in the background.

This is an Operate-mode tool, used daily and at a glance, so density stays moderate and the
layout is predictable. The aim for components is **soft and approachable** — developers
should feel the app is helping them, not auditing them, even on screens that ask for trust
(capability consent, lint errors). That shows up as 6px corners, white widgets separated
from the page by a single soft shadow, a low-alpha accent wash that marks the current place,
and gentle 150ms transitions.

Depth here is earned by **inversion plus one shadow**, not by boxes stacked on boxes. The
page recedes, the widgets advance, and a single ambient beam gives the frame its one moment
of atmosphere. The reduced-motion policy that earlier versions left open is now settled (see
Elevation & Depth and Layout). Appearance follows the OS by default and can be pinned to
light or dark from Settings.

**Key Characteristics:**
- One accent (Signal Blue), now with a low-alpha wash (`accent-wash`) for the current place.
- Inverted surfaces: a tinted page, white widgets that lift.
- Light and dark follow `prefers-color-scheme`, overridable with `data-theme` on `<html>`.
- System UI type. Mono for anything a machine reads: inputs, paths, hosts, code.
- One shadow family, for overlays and widget hover only. One ambient beam, app-level.
- A single radius (6px) on every enclosed shape; 4px on segments nested inside one.
- Navigation lives in a quiet white sidebar beside the page; the page itself is all data.

## Colors

A cool neutral palette with one confident blue signal, a low-alpha wash of that same blue,
and two plain state colors.

### Primary
- **Signal Blue** (light / dark: `signal-blue` / `signal-blue-dark`): the lamp. Primary
  button fill, the 3px bar on the current sidebar item, link buttons, focus outlines, and the
  heading of the "needs setup" widget state. Text on a blue fill uses **Signal Ink**
  (`signal-ink` / `signal-ink-dark`).
- **Accent Wash** (`accent-wash` / `accent-wash-dark`): Signal Blue at ~9–15% alpha. The
  soft presence of the lamp — behind the current sidebar item and the selected theme segment.
- **Accent Weak** (`accent-weak` / `accent-weak-dark`): Signal Blue at ~24–30% alpha. The
  border a widget takes on hover, and the fading top hairline of the beam.

### Neutral
- **Page** (`page` / `page-dark`): the tinted page background — cool off-white in light,
  near-black in dark. The surface everything else sits on; it recedes so widgets advance.
- **Open White / Deep Slate** (`open-white` / `deep-slate-surface`): the raised surface for
  the sidebar, the narrow top bar, widgets, cards, notices, default buttons, inputs, dialogs and menus. White on the tint in
  light; one soft step above the page in dark.
- **Mist** (`mist-surface` / `mist-surface-dark`): a faint tonal step above the surface, for
  sticky table headers, hovered table rows, hovered sidebar items and the monogram tiles.
- **Ink / Moon Gray** (`ink` / `moon-gray`): body and heading text.
- **Slate Muted** (`slate-muted` / `slate-muted-dark`): secondary text, table headers, field
  labels, inactive sidebar items, sidebar group headings, empty and loading states. The most-used text class after body text.
- **Fog Line** (`fog-line` / `fog-line-dark`): every outer 1px border — widgets, cards,
  notices, inputs, dialogs, menus, the sidebar's inner edge and the narrow bar's bottom edge.
- **Fog Soft** (`fog-soft` / `fog-soft-dark`): inner dividers and table rules — the widget
  header rule, table row lines, section separators. Softer than
  Fog Line so outer edges stay the firmer line.

### State
- **Buoy Green** (`buoy-green` / `buoy-green-dark`): confirmations such as `✓ git version …`.
  Used only as text color.
- **Flare Red** (`flare-red` / `flare-red-dark`) with **Flare Wash** (`flare-wash` /
  `flare-wash-dark`): errors and destructive warnings. Danger notices take the wash
  background and a red border, with the heading in red and body text in Ink. Text on a
  red fill (the danger button) uses **Flare Ink** (`flare-ink` / `flare-ink-dark`).

### Named Rules
**The One Lamp Rule.** Signal Blue appears only where it marks the current place, the next
action or a needed setup step. A screen has at most one primary button.

**The Halo Rule.** The accent may appear as a low-alpha wash (`accent-wash`) or weak border
(`accent-weak`) in addition to its solid fill — but only where the One Lamp Rule already
allows it: the current place (the current sidebar item with its 3px Signal Blue bar, the
selected segment), a hovered widget, focus. Never
as decoration, and never a second hue.

**The Paired Theme Rule.** Each color token has a dark-mode value. System appearance follows
`prefers-color-scheme`; an explicit choice is pinned with `data-theme="light|dark"` on
`<html>`, which overrides the OS. A new color isn't finished until both values exist.

## Typography

**Body Font:** system-ui (with -apple-system, sans-serif)
**Label/Mono Font:** ui-monospace (with "SF Mono", Menlo, monospace)

**Character:** The OS's own voice for words, and a monospace for what machines read. The
pairing makes the app feel native on macOS and Linux, and makes paths, hosts and commands
obviously exact.

### Hierarchy
- **Headline** (700, 1.5rem, 1.45): the screen title, `h1`, on Onboarding.
- **Page Title** (650, 1.35rem, -0.015em): the `h1` of the slim page header — the dashboard's
  title, "Settings", "Dashboards" on an empty list, and the platform title before the first
  sync — set below Headline on purpose so the data leads.
- **Title** (700, 1.15rem): section titles (`h2`) and dialog titles (1.1rem).
- **Title Small** (700, 1rem): `h3`, card and markdown headings.
- **Body** (400, 1rem, 1.45): default text. Screens cap their width at 44rem.
- **Body Small** (400, 0.9rem): tables and `dl` field lists inside widgets.
- **Label** (500, 1rem): form labels, table headers, capability terms, segmented options.
  The current sidebar item uses 600.
- **Nav Heading** (600, 0.72rem, 0.06em tracking, uppercase, Slate Muted): the *Platform* and
  *Mine* group headings in the sidebar.
- **Mono** (400, 0.85rem; inputs 0.9rem): code, `pre`, every text input, and capability
  values such as hosts, executables and paths.

### Named Rules
**The Machine Text Rule.** Text that a developer could paste into a terminal or a config
file is set in mono. Prose never is.

## Layout

The spacing scale has four steps: 0.25rem, 0.5rem, 1rem and 2rem (`space-1` to `space-4`).
Layout uses flex `stack` (column) and `row` utilities with a 1rem gap. Lists use a 0.5rem
gap, and labels sit 0.25rem above their fields.

- **Form and review screens** (Onboarding, review, Settings): one column, max 44rem, 2rem
  padding. Settings fields cap at 32rem. Sections are separated by a Fog Soft rule and 1rem
  of padding.
- **App shell:** a two-column grid — the sidebar (240px open, 56px as a rail) beside the
  main area (padding 1rem by 2rem, 1rem gaps). The sidebar is sticky at full window height
  and scrolls inside itself; its width change eases over 150ms under the motion guard. Above
  the dashboards sit, in order, any sync review, changelog or sync error, then the page header.
  Settings renders in the main area with the sidebar still visible.
- **Dashboards:** a slim page header holds the dashboard title, a muted origin line
  (*Platform dashboard · read-only* or *Your dashboard*) and that dashboard's actions, which
  wrap below the title when space runs out. Then a **12-column grid** with 80px auto rows and 1rem gaps, placed from dashboard coordinates. A container
  widget lays its own parts out on a nested 12-column grid (64px rows) inside its body.
- **Card grids** inside widgets: `auto-fill`, minimum 14rem per column, 0.5rem gaps.
- **Breakpoints:** at 1280px and wider the sidebar starts open, below it starts as the rail;
  a saved choice wins either way. At 900px and below the sidebar gives way to the narrow top
  bar.
- **Responsive:** at 900px and below the dashboard grid collapses to one column, the main
  area's side padding drops to 1rem, top-level widgets drop their placed coordinates and cap at
  24rem tall (scrolling inside); a composite's nested parts size to content and scroll once
  within that cap. Dialogs shrink to `min(28rem, 100%)` and scroll in a short window.
- **Motion:** state changes on buttons, widgets, sidebar items and table rows ease over 150ms, inside
  a `prefers-reduced-motion: no-preference` guard. Under reduced motion there are no
  transitions and the loading skeleton holds still (its bars still read as loading).

## Elevation & Depth

Depth is built from two moves: a **surface inversion** (the page recedes as a cool tint,
white widgets advance) and **one shadow family**, used sparingly. Overlays — dialogs and the
overflow/catalog menus — take `shadow-pop` over a 40% scrim. A widget rests on the faint
`shadow-card` and, on hover, rises to `shadow-pop` with an `accent-weak` border and a 2px
lift. Nothing else casts a shadow: notices, cards, inputs and nested composite parts stay
flat, separated by tone and a Fog border.

Above all this sits **the beam**: one ambient radial glow of Signal Blue at the top of the
app frame (a fixed `body::before`, ~220px tall), with a faint `accent-weak` hairline along
the very top edge that fades toward the center. It is the lighthouse, and it appears exactly
once — never repeated per widget or per section.

### Shadow Vocabulary
- **shadow-card** (`0 1px 2px rgba(20,30,55,.05), 0 2px 6px rgba(20,30,55,.06)`; dark
  `0 1px 2px rgba(0,0,0,.35), 0 2px 8px rgba(0,0,0,.30)`): a widget at rest.
- **shadow-pop** (`0 12px 34px rgba(20,30,55,.16)`; dark `0 14px 38px rgba(0,0,0,.55)`):
  overlays (dialog, menu) and a widget on hover.

### Named Rules
**The Tone First, One Shadow Rule.** Depth comes first from the surface inversion and a
tonal step; a shadow is allowed only for overlays and widget hover, never elsewhere. If a
resting surface needs separating, use tone and a Fog border, not a shadow.

**The Beam Rule.** Exactly one ambient expression of the lighthouse, at the top of the app
frame. Never repeated on a widget, a card or a section.

## Shapes

One gently rounded corner (6px) on every enclosed shape: buttons, inputs, cards, widget
frames, notices, dialogs and menus. Segments nested inside a control (the theme switcher's
options inside its track) use a tighter 4px so the inner radius reads as a child of the
outer. Borders are always 1px solid. The exceptions are link buttons, which have no box at
all, and the current sidebar item's marker, a 3px Signal Blue bar on its inner edge with its
outer corners rounded to 3px.

## Components

### Buttons
Quiet by default; only the next step is lit.
- **Shape:** gently rounded (6px), 1px border, padding 0.5rem by 1rem, inheriting the body
  font.
- **Default:** Open White fill, Ink text, Fog Line border. Used for Discard, Sync now, Cancel
  and most actions.
- **Primary:** Signal Blue fill and border with Signal Ink text. Used for Connect,
  Approve and apply, and Save.
- **Danger:** Flare Red fill and border with Flare Ink text. Used only for the confirming
  action inside a destructive dialog (Remove, Disconnect), never for the button that opens
  that dialog.
- **Link:** no box, Signal Blue text. Used for inline navigation such as "Review the repo"
  and the widget-header Remove action.
- **Focus:** 2px Signal Blue outline with a 1px offset (`:focus-visible`).
- **Disabled:** 50% opacity, default cursor.
- **Hover:** one tone deeper, over 150ms. Default buttons fill with Fog Line; Primary and
  Danger darken slightly (`brightness(0.92)`); Link buttons underline.

### Sidebar (navigation)
The way around the app, kept quiet so the dashboards lead. An Open White column beside the
tinted page with a Fog Line border on its inner edge and no shadow. From the top: the brand
(the mark and the platform title) with the source as `owner/repo · branch` in small muted
text; the collapse toggle; a **Platform** group and a **Mine** group of dashboards, each its
own `nav` landmark under a Nav Heading, with **+ New dashboard** last in Mine; and a footer
pinned to the bottom with the sync status (an always-present live region), the short commit,
**Sync now** and **Settings**.
- **Item:** 6px radius, no border, 6px by 0.5rem padding, a 22px monogram tile (the title's
  first letter, Mist fill, Fog Line border) and the title, ellipsised. The landing dashboard
  carries a star.
- **Rest / Hover:** Slate Muted text and no fill; hover lifts it to Ink on a Mist fill.
- **Current** (`aria-current="page"`): Ink, weight 600, Accent Wash fill and the 3px Signal
  Blue bar on the inner edge (the Halo Rule).
- **Focus:** the standard 2px Signal Blue outline.
- **Rail (56px):** only the mark, monograms, a ↻ sync button and the settings icon show.
  Labels stay in the accessibility tree (visually hidden, never removed), and the item's
  title appears in a small Open White tooltip on hover and on keyboard focus. ⌘\ or Ctrl+\
  toggles open and rail, except while typing or in a dialog; the choice is saved per device.
- **Narrow top bar (900px and below):** the sidebar is replaced by an Open White bar with a
  Fog Line bottom edge: the mark and the current dashboard as a menu button that opens the
  shared overflow menu (both group headings and their dashboards), then Sync now and
  Settings.

### Segmented Control (theme switcher)
- The Appearance control in Settings: Light / Dark / System. Native radios (so arrow keys
  and screen readers work) styled as joined segments inside an Open White, Fog-bordered
  track. The selected segment takes the **Accent Wash** and Ink text (the Halo Rule); the
  radio itself is visually hidden but keyboard-reachable, and focus shows the 2px outline on
  its segment.

### Cards / Containers
- **Card:** 6px radius, Fog Line border, Open White background, 1rem padding, no shadow. Used
  for widget list items and card views.
- **Widget frame:** Open White fill, 6px radius, Fog Line border and a resting `shadow-card`.
  The header strip has 0.5rem by 1rem padding and a Fog Soft bottom rule. The body scrolls
  independently with 1rem padding. A top-level widget rises on hover (`shadow-pop`,
  `accent-weak` border, 2px lift); a composite's nested parts are flat Fog Soft panes.
- **Widget states** (shared by every widget): text centered both ways in Slate Muted.
  Empty and loading states use muted text plus a three-bar pulsing skeleton (still under
  reduced motion). The error state has a Flare Red heading. The setup-required state has a
  Signal Blue heading and links to Settings. Names a state refers to (widgets, inputs,
  secrets, paths) are mono in Ink, and error details stay left-aligned as a log.

### Notices
- **Default:** Open White fill, Fog Line border, 6px radius, 1rem padding. Used for neutral
  status such as "Already up to date".
- **Danger:** Flare Wash fill, Flare Red border and a red `strong` heading. Used for git
  and clone errors and for lint errors, which list diagnostics with their file and field.

### Inputs / Fields
- **Style:** text inputs and selects share one style: mono 0.9rem, Open White background, Fog
  Line border, 6px radius, 0.5rem padding, so they stay crisp on the tinted page. The label
  is weight 500 and stacks 0.25rem above. Transient status for the field ("Saved") sits at
  the right of the label row, so it never shifts the layout.
- **Checkbox:** 1rem with the accent color, beside its text in a regular-weight sentence row.
- **Focus:** 2px Signal Blue outline with a 1px offset.
- **Error:** a Flare Red text line (`.danger-text`) or a danger notice below the field,
  linked with `aria-describedby` and `aria-invalid`.

### Tables
- Full width, collapsed borders, 0.9rem. Row lines are Fog Soft. The header (`thead th`) is
  sticky to the scrolling widget body with a Mist background; `tbody tr` takes a Mist
  background on hover.

### Capability list (signature)
The trust surface. A two-column `dl`: the verb (*Run*, *Call*, *Read*) at weight 500, and
the value (the executable, host or repo path) in mono. Each group is headed by the widget
it belongs to. It must stay readable at a glance, because developers approve exactly what
it shows.

### Dialog
- An in-app modal (the browser's `prompt()` and `alert()` are never used). `min(28rem, 100%)`
  wide, Open White, Fog Line border, 6px radius, `shadow-pop`, over a 40% black backdrop.
  Focus moves to the dialog on open, is trapped inside while open, and returns to the trigger
  on close; Escape closes. Actions sit right-aligned, Cancel first and the destructive action
  last. Every destructive removal asks here first.

## Do's and Don'ts

### Do:
- **Do** use the tokens (`--accent`, `--accent-wash`, `--surface`, `--border`, `--border-soft`,
  `--shadow-card`, `--space-*`, `--radius`) rather than literal values, and define every new
  color for both themes.
- **Do** keep a single primary (Signal Blue) button per view. Make everything else a
  default or link button.
- **Do** keep the page tinted and the widgets Open White, so widgets lift off the page.
- **Do** set paths, hosts, commands, SHAs and inputs in mono (`--mono`).
- **Do** reuse the shared widget states (empty, loading, error, needs setup) rather than
  designing new ones for each widget.

### Don't:
- **Don't** add a shadow outside the two allowed places (overlays, widget hover). Resting
  surfaces separate by tone and a Fog border.
- **Don't** repeat the beam, or add a second ambient glow. It belongs once, at the top of the
  frame.
- **Don't** introduce a second accent hue, or use Signal Blue — solid or washed — for
  decoration rather than to mark place, action or setup.
- **Don't** load remote fonts, icon fonts or CDN assets. The strict CSP allows only the
  app's own scripts and styles.
- **Don't** use browser `prompt()` or `alert()`. Use the in-app Dialog.
- **Don't** let the platform's branding replace the Pharos frame. Platform titles appear as
  content.
