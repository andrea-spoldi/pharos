---
name: session-planner
description: >
  Use when the user says "plan my session", "what should I work on next", "start
  a session", "let's work on the project", "checkpoint", "compact", "close session",
  "end of day", "add a task", "update my backlog", "reprioritize tasks", or "audit
  my CLAUDE.md". Also trigger when the user opens a repo and seems unsure where to
  start, when context is getting long and they haven't compacted yet, or when they
  ask anything about managing token usage or Claude Code productivity.
---

# Session Planner

A meta-skill for structuring Claude Code sessions to maximise token efficiency
and preserve continuity across session boundaries.

It operates **outside** the work itself — at the edges: before, during checkpoints,
and at the close. It does not write code.

-----

## Philosophy

- **One session = one atomic unit of work.** Never start a session without a defined goal.
- **Context is the budget.** Load only what's needed. Compact at natural checkpoints.
- **Decisions outlive sessions.** Write them down before compacting or closing.
- **Token spend must match task complexity.** Disable extended thinking for mechanical work.
- **CLAUDE.md is overhead on every turn.** Keep it under 50 lines, always.

-----

## Skill State

State is persisted in **`TASKS.md`** at the repo root. Always read it first.
If it doesn't exist → run **Onboarding Mode**.
If it exists → run **Session Start Mode**.

The canonical state format lives in `references/tasks-schema.md`.

-----

## Operating Modes

### 0. Onboarding Mode

*Triggered when `TASKS.md` does not exist.*

1. Ask the user for a one-line project description.
1. Ask for 3–5 tasks they want to tackle in the coming sessions. Size each as S / M / L.
1. Flag any **L** tasks immediately and ask to decompose before proceeding.
1. Create `TASKS.md` using the schema from `references/tasks-schema.md`.
1. Create a lean `CLAUDE.md` using the template from `references/claude-md-template.md`.
1. Transition to **Session Start Mode**.

-----

### 1. Session Start Mode

*Triggered by: "plan my session", "what's next", "let's start", opening a repo.*

**Steps:**

1. Read `TASKS.md`. Show current backlog, highlight the top `pending` task.
1. Confirm with the user or let them pick a different task.
1. Set `current_session.goal` and `status: in-progress` in `TASKS.md`.
1. Run the **Pre-Session Hygiene Checklist** (see below).
1. Write updated `TASKS.md`.
1. Output a one-line session contract:

```
🎯 Session goal: [goal]
📦 Estimated size: [S/M/L]
🧹 Hygiene: [any flags from checklist, or "clean"]
```

**Pre-Session Hygiene Checklist:**

|Check                    |Action                                                                                                                                    |
|-------------------------|------------------------------------------------------------------------------------------------------------------------------------------|
|`CLAUDE.md` > 50 lines?  |Flag stale rules. Offer to trim.                                                                                                          |
|Task is L-sized?         |Refuse to start. Ask to decompose first.                                                                                                  |
|Extended thinking needed?|If task is mechanical (formatting, renaming, boilerplate), suggest disabling: `/config` → thinking off, or set `MAX_THINKING_TOKENS=8000`.|
|Many MCPs connected?     |List active ones. Suggest disabling any not needed for this task.                                                                         |
|Last session > 1hr ago?  |Warn: prompt cache likely expired. Suggest `/clear` for a fresh start.                                                                    |

-----

### 2. Checkpoint Mode

*Triggered by: "checkpoint", "compact", task unit complete, natural pause.*

Use this after completing a discrete sub-task, before moving to the next one.
Think of it as a git commit for context.

**Steps:**

1. Ask Claude to emit a decision record for anything non-obvious decided so far.
1. Update `TASKS.md`: mark completed sub-steps, note blockers, update `current_session`.
1. Output a **Compact Primer** for the user to prepend to `/compact`:

```
Before compacting, preserve:
──────────────────────────────────────────
Current session goal : [goal]
Progress so far      : [what was done]
Key decisions        : [D-xxx, or inline]
Next step            : [concrete action after compact]
Files modified       : [list]
Do NOT discard       : [anything specific Claude must keep]
──────────────────────────────────────────
```

1. Tell the user: "Paste the above as your first message to `/compact`, then run it."

-----

### 3. Session Close Mode

*Triggered by: "close session", "we're done", "end of day", "wrapping up".*

**Steps:**

1. Summarise what was completed this session (1–3 lines).
1. Update `TASKS.md`:
   - Move `current_session` task to `completed[]` with a date.
   - Update backlog priorities if needed.
   - Log any new decisions to `decisions[]`.
1. Suggest the next session goal based on remaining backlog.
1. Emit a **Next Session Primer** (a one-paragraph context note the user can paste at
   the start of the next session to restore context cheaply without re-reading history).
1. Remind: if returning after > 1 hour, run `/clear` and paste the primer fresh.

**Next Session Primer format:**

```
## Context for next session
Project  : [project name]
Last done: [what was completed]
Next goal: [next task title and ID]
Key state: [any live facts Claude needs — branch name, env, decisions in play]
Ref files: [TASKS.md, and any specific files to load]
```

-----

### 4. Backlog Management Mode

*Triggered by: "add a task", "reprioritize", "update backlog", "remove task".*

Simple CRUD on the `TASKS.md` backlog.

**Task sizing rules:**

- **S** — single focused change, < 30 min, one file or one function scope
- **M** — a feature or coherent refactor, 30–90 min, a few files
- **L** — multi-part, should be decomposed before scheduling

Always flag **L** tasks and decompose them into S/M before adding to the active queue.
Never schedule an **L** task directly.

-----

### 5. CLAUDE.md Audit Mode

*Triggered by: "audit my CLAUDE.md", "is my CLAUDE.md lean", or during Session Start if file > 50 lines.*

Load `references/claude-md-template.md` for the canonical lean structure.

**Audit rules:**

|Category                            |Rule                                                                                |
|------------------------------------|------------------------------------------------------------------------------------|
|Default behaviours                  |Remove. Claude writes clean code, uses meaningful names, follows style — by default.|
|Stale history                       |Remove. Past migrations, deprecated paths, old decisions no longer in play.         |
|Non-obvious architecture constraints|**Keep.** These are the reason the file exists.                                     |
|Security rules                      |**Keep.** Place them at the top, never remove.                                      |
|Counterintuitive decisions          |**Keep.** "We use CSR here intentionally" saves repeated conversations.             |

Target: **under 50 lines**. Every line costs tokens on every turn in this repo.

-----

## Output Format Rules (enforce in all modes)

These apply to the coding work Claude does within the session, not just planning:

- **Always request diffs, not full rewrites**, for any file > 50 lines. Tell Claude: *"Output as a diff, not the full file."*
- **Minimal working version first.** Ask for the bare implementation, then iterate. Avoid asking Claude to speculate about all edge cases upfront.
- **Scope prompts explicitly.** Always name the file, function, or layer in scope. Vague prompts lead to over-generation.
- **One concern per prompt.** Don't chain multiple changes in a single message.

-----

## Reference Files

Load these only when needed — do not load all by default:

- `references/tasks-schema.md` — Full `TASKS.md` JSON schema and example
- `references/claude-md-template.md` — Canonical lean `CLAUDE.md` template

-----

## Quick Reference Card

```
Session start  → read TASKS.md → pick task → hygiene check → session contract
Mid-session    → checkpoint after each sub-task → compact primer → /compact
Session close  → summarise → update TASKS.md → next session primer
Backlog        → size tasks S/M/L → decompose all L before scheduling
CLAUDE.md      → < 50 lines → only non-obvious constraints + security rules
Prompting      → diffs not rewrites → minimal first → one concern per turn
```
