---
name: Pharos
description: A calm desktop frame for a platform team's dashboards, with one blue signal where attention belongs.
colors:
  signal-blue: "#2f5bd3"
  signal-ink: "#ffffff"
  open-white: "#ffffff"
  mist-surface: "#f5f6f8"
  ink: "#1a1a1a"
  slate-muted: "#5f6672"
  fog-line: "#d8dce3"
  buoy-green: "#1e7a46"
  flare-red: "#b42318"
  flare-wash: "#fdf1f0"
  flare-ink: "#ffffff"
  signal-blue-dark: "#6b8cff"
  signal-ink-dark: "#0d1020"
  harbour-night: "#16181d"
  deep-slate-surface: "#1f2229"
  moon-gray: "#e6e6e6"
  slate-muted-dark: "#9aa1ad"
  fog-line-dark: "#343843"
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
spacing:
  space-1: "0.25rem"
  space-2: "0.5rem"
  space-3: "1rem"
  space-4: "2rem"
components:
  button-default:
    backgroundColor: "{colors.mist-surface}"
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
  tab:
    textColor: "{colors.slate-muted}"
    padding: "0.5rem 1rem"
  tab-active:
    textColor: "{colors.ink}"
    padding: "0.5rem 1rem"
  input:
    backgroundColor: "{colors.open-white}"
    textColor: "{colors.ink}"
    typography: "{typography.mono-input}"
    rounded: "{rounded.base}"
    padding: "0.5rem"
  card:
    rounded: "{rounded.base}"
    padding: "1rem"
  widget-frame:
    backgroundColor: "{colors.mist-surface}"
    rounded: "{rounded.base}"
  notice:
    backgroundColor: "{colors.mist-surface}"
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

Pharos is named after a lighthouse, and its interface works like one. The frame stays still
and quiet so that one signal stands out. That signal is a single blue, kept for the places
where the developer's attention belongs: the current tab, the one primary action on a
screen, and the prompt that says a widget needs setup. Everything else is neutral: system
type, pale tonal surfaces, thin fog-colored borders. The platform team's own content and
data fill the frame; Pharos itself stays in the background.

This is an Operate-mode tool, used daily and at a glance, so density stays moderate and the
layout is predictable. The aim for components is **soft and approachable**. Developers
should feel the app is helping them, not auditing them, even on screens that ask for trust
(capability consent, lint errors). Today that shows up as 6px corners, tonal surfaces in
place of hard boxes, and gray default buttons. A future refinement may push the softness
further.

**This is a provisional baseline.** It accurately records the system as it stands at
slice 5, which is still close to the scaffold. Tokens and the rules below hold until they
are changed on purpose. These areas are explicitly open: the type scale (there's no
display role, and headings use browser-default weights), how far the accent can extend
beyond its current uses, a warning/caution color distinct from danger, and a
reduced-motion policy.

**Key Characteristics:**
- One accent (Signal Blue). Everything else is neutral or a state color.
- Light and dark themes follow `prefers-color-scheme`, defined as token overrides on `:root`.
- System UI type. Mono for anything a machine reads: inputs, paths, hosts, code.
- Flat surfaces separated by tone and 1px borders. No shadows.
- A single radius (6px) on every enclosed shape.

## Colors

A cool neutral palette with one confident blue signal and two plain state colors.

### Primary
- **Signal Blue** (light / dark: `signal-blue` / `signal-blue-dark`): the lamp. Primary
  button fill, the underline under the active tab, link buttons, focus outlines, and the
  heading of the "needs setup" widget state. Text on a blue fill uses **Signal Ink**
  (`signal-ink` / `signal-ink-dark`).

### Neutral
- **Open White / Harbour Night** (`open-white` / `harbour-night`): the page background.
  Also used for inputs, dialogs and menus, which sit at page level.
- **Mist / Deep Slate** (`mist-surface` / `deep-slate-surface`): the tonal surface for
  widget frames, notices and default buttons. This one step of tone does the separating
  that a shadow would do elsewhere.
- **Ink / Moon Gray** (`ink` / `moon-gray`): body and heading text.
- **Slate Muted** (`slate-muted` / `slate-muted-dark`): secondary text, table headers, field
  labels in `dl` lists, inactive tabs, empty and loading states. It's the most-used text
  class after body text.
- **Fog Line** (`fog-line` / `fog-line-dark`): every 1px border, table rule and divider,
  plus the skeleton bars.

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

**The Paired Theme Rule.** Each color token has a dark-mode value, overridden on `:root`
under `prefers-color-scheme: dark`. A new color isn't finished until both values exist.

## Typography

**Body Font:** system-ui (with -apple-system, sans-serif)
**Label/Mono Font:** ui-monospace (with "SF Mono", Menlo, monospace)

**Character:** The OS's own voice for words, and a monospace for what machines read. The
pairing makes the app feel native on macOS and Linux, and makes paths, hosts and commands
obviously exact.

### Hierarchy
- **Headline** (700, 1.5rem, 1.45): the screen title, `h1`, on Onboarding and Settings.
- **Title** (700, 1.15rem): section titles (`h2`), dialog titles (1.1rem) and the dashboard
  top bar's `h1`, which is set smaller on purpose so the data leads.
- **Title Small** (700, 1rem): `h3`, card and markdown headings.
- **Body** (400, 1rem, 1.45): default text. Screens cap their width at 44rem.
- **Body Small** (400, 0.9rem): tables and `dl` field lists inside widgets.
- **Label** (500, 1rem): form labels, table headers and capability terms. The active tab
  uses 600.
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
  padding. Settings fields cap at 32rem. Sections are separated by a Fog Line rule and 1rem
  of padding.
- **Dashboards** (`.screen.wide`): full width, padding 1rem by 2rem. A top bar holds the
  title and actions. Sync progress is muted text beside the actions, never a line that
  pushes the dashboard down, and the actions drop below the title when space runs out. Then a tab strip with a bottom border, then a **12-column grid** with
  80px auto rows and 1rem gaps, placed from dashboard YAML coordinates.
- **Card grids** inside widgets: `auto-fill`, minimum 14rem per column, 0.5rem gaps.
- **Responsive:** below 900px the dashboard grid collapses to one column, widgets drop
  their placed coordinates, and each widget caps at 24rem tall and scrolls inside.

## Elevation & Depth

The system is flat. There are no `box-shadow` values anywhere. Depth comes from one tonal
step: page-level surfaces use Open White, and contained surfaces (widgets, notices) use
Mist. A 1px Fog Line border holds both. Overlays separate by z-index and a backdrop rather
than by lift: dialogs sit over a 40% black scrim, and the overflow menu sits on the page
background with a border.

### Named Rules
**The Tone Not Shadow Rule.** Separate a surface with a tonal step or a Fog Line border,
never a shadow. If something needs to feel raised, it's an overlay and gets a scrim.

## Shapes

One gently rounded corner (6px) on every enclosed shape: buttons, inputs, cards, widget
frames, notices, dialogs, menus and even skeleton bars. Borders are always 1px solid.
The two exceptions are tabs, which are square with a 2px bottom indicator, and link
buttons, which have no box at all.

## Components

### Buttons
Quiet by default; only the next step is lit.
- **Shape:** gently rounded (6px), 1px border, padding 0.5rem by 1rem, inheriting the body
  font.
- **Default:** Mist fill, Ink text, Fog Line border. Used for Discard, Sync now, Cancel and
  most actions.
- **Primary:** Signal Blue fill and border with Signal Ink text. Used for Connect,
  Approve and apply, and Save.
- **Danger:** Flare Red fill and border with Flare Ink text. Used only for the confirming
  action inside a destructive dialog (Remove, Disconnect), never for the button that opens
  that dialog.
- **Link:** no box, Signal Blue text. Used for inline navigation such as "Review the repo"
  and back links.
- **Focus:** 2px Signal Blue outline with a 1px offset (`:focus-visible`).
- **Disabled:** 50% opacity, default cursor.
- **Hover:** one tone deeper, never a shadow. Default buttons fill with Fog Line; Primary and
  Danger darken slightly (`brightness(0.92)`); Link buttons underline. Tabs and menu rows
  keep their own hover.

### Tabs
- Dashboard switcher. Inactive tabs have Slate Muted text and no box. The active tab
  (`aria-current="page"`) has Ink text, weight 600 and a 2px Signal Blue bottom border.
  Hover lifts inactive tabs to Ink. Extra dashboards go into a "More" overflow menu (min
  12rem, page background, Fog Line border) whose rows are full-width Ink text with a Mist
  hover, and the current dashboard is set in 600.

### Cards / Containers
- **Card:** 6px radius, Fog Line border, page background, 1rem padding. Used for widget
  list items and card views.
- **Widget frame:** Mist fill, 6px radius and Fog Line border. The header strip has
  0.5rem by 1rem padding and a bottom rule. The body scrolls independently with 1rem
  padding.
- **Widget states** (shared by every widget): text centered both ways in Slate Muted.
  Empty and loading states use muted text plus a three-bar pulsing skeleton. The error
  state has a Flare Red heading. The setup-required state has a Signal Blue heading and
  links to Settings. Names a state refers to (widgets, inputs, secrets, paths) are mono in
  Ink so they stand out from the muted text, and error details stay left-aligned as a log.

### Notices
- **Default:** Mist fill, Fog Line border, 6px radius, 1rem padding. Used for neutral
  status such as "Already up to date".
- **Danger:** Flare Wash fill, Flare Red border and a red `strong` heading. Used for git
  and clone errors and for lint errors, which list diagnostics with their file and field.

### Inputs / Fields
- **Style:** text inputs and selects share one style: mono 0.9rem, page background, Fog
  Line border, 6px radius, 0.5rem padding. The label is weight 500 and stacks 0.25rem
  above. Transient status for the field ("Saved") sits at the right of the label row, so it
  never shifts the layout.
- **Checkbox:** 1rem with the accent color, set beside its text in a regular-weight row
  label that reads as a sentence.
- **Focus:** 2px Signal Blue outline with a 1px offset.
- **Error:** shown as a Flare Red text line (`.danger-text`) or a danger notice below the
  field, linked to the control with `aria-describedby` and `aria-invalid`. There is no red
  field border yet.

### Capability list (signature)
The trust surface. A two-column `dl`: the verb (*Run*, *Call*, *Read*) at weight 500, and
the value (the executable, host or repo path) in mono. Each group is headed by the widget
it belongs to. It must stay readable at a glance, because developers approve exactly what
it shows.

### Dialog
- An in-app modal (the browser's `prompt()` and `alert()` are never used). Max width 28rem,
  page background, Fog Line border, 6px radius, padding 1rem by 2rem, over a 40% black
  backdrop. Actions sit right-aligned in a row, Cancel first and the destructive action
  last, at the trailing edge. Every destructive removal asks here first.

## Do's and Don'ts

### Do:
- **Do** use the existing tokens (`--accent`, `--surface`, `--border`, `--space-*`,
  `--radius`) rather than literal values, and define every new color for both themes.
- **Do** keep a single primary (Signal Blue) button per view. Make everything else a
  default or link button.
- **Do** set paths, hosts, commands, SHAs and inputs in mono (`--mono`).
- **Do** separate surfaces with a tonal step (Open White and Mist) and 1px Fog Line borders.
- **Do** reuse the shared widget states (empty, loading, error, needs setup) rather than
  designing new ones for each widget.

### Don't:
- **Don't** add box shadows. Depth is tonal, and overlays get a scrim.
- **Don't** introduce a second accent hue, or use Signal Blue for decoration.
- **Don't** load remote fonts, icon fonts or CDN assets. The strict CSP allows only the
  app's own scripts and styles.
- **Don't** use browser `prompt()` or `alert()`. Use the in-app Dialog.
- **Don't** let the platform's branding replace the Pharos frame. Platform titles appear as
  content.
