# TASKS.md Schema Reference

The `TASKS.md` file at the repo root is the session-persistent state store.
It is both human-readable and machine-parseable. Use the JSON block as the
canonical state; the markdown sections are for human readability.

-----

## Full Schema

```json
{
  "project": "string — repo name or short identifier",
  "updated": "ISO 8601 date — last time this file was written",

  "current_session": {
    "id": "string — e.g. S-007",
    "goal": "string — one-line description of what this session accomplishes",
    "task_ref": "string — ID of the backlog task being worked on, e.g. T-003",
    "started": "ISO 8601 date",
    "status": "in-progress | done | blocked",
    "blocker": "string | null — describe the blocker if status is blocked"
  },

  "backlog": [
    {
      "id": "string — e.g. T-001",
      "title": "string — short imperative description",
      "description": "string | null — optional detail",
      "size": "S | M | L",
      "priority": "integer — 1 is highest",
      "status": "pending | in-progress | done | cancelled",
      "tags": ["string"]
    }
  ],

  "decisions": [
    {
      "id": "string — e.g. D-001",
      "date": "ISO 8601 date",
      "decision": "string — what was decided",
      "rationale": "string — why, especially if non-obvious",
      "supersedes": "string | null — ID of a previous decision this replaces"
    }
  ],

  "completed": [
    {
      "id": "string — task ID from backlog",
      "title": "string",
      "completed_date": "ISO 8601 date",
      "session_ref": "string — session ID when this was completed",
      "notes": "string | null"
    }
  ]
}
```

-----

## Annotated Example

```json
{
  "project": "treed",
  "updated": "2026-04-22",

  "current_session": {
    "id": "S-003",
    "goal": "Add --max-depth flag to the root command",
    "task_ref": "T-002",
    "started": "2026-04-22",
    "status": "in-progress",
    "blocker": null
  },

  "backlog": [
    {
      "id": "T-001",
      "title": "Add --ignore-hidden flag",
      "description": "Skip dotfiles and dot-directories when rendering the tree",
      "size": "S",
      "priority": 1,
      "status": "done",
      "tags": ["cli", "filtering"]
    },
    {
      "id": "T-002",
      "title": "Add --max-depth flag",
      "description": "Limit tree rendering to N levels deep",
      "size": "S",
      "priority": 2,
      "status": "in-progress",
      "tags": ["cli", "filtering"]
    },
    {
      "id": "T-003",
      "title": "Add --output-format flag (json | ascii)",
      "description": null,
      "size": "M",
      "priority": 3,
      "status": "pending",
      "tags": ["cli", "output"]
    }
  ],

  "decisions": [
    {
      "id": "D-001",
      "date": "2026-04-20",
      "decision": "Use Cobra for CLI, not flag package",
      "rationale": "Consistent with existing Go CLI conventions in personal repos. Cobra gives subcommand support for free if needed later.",
      "supersedes": null
    },
    {
      "id": "D-002",
      "date": "2026-04-21",
      "decision": "Render symlinks as [-> target] inline, not followed",
      "rationale": "Avoids infinite loops and keeps output predictable. Following symlinks is opt-in via a future flag.",
      "supersedes": null
    }
  ],

  "completed": [
    {
      "id": "T-001",
      "title": "Add --ignore-hidden flag",
      "completed_date": "2026-04-21",
      "session_ref": "S-002",
      "notes": "Implemented via WalkDir filter. Added unit test for dotfile skipping."
    }
  ]
}
```

-----

## Usage Notes

- **IDs are sequential and never reused.** Completed tasks stay in `completed[]`,
  not deleted, so decisions and session refs remain traceable.
- **`current_session` is overwritten each session start.** Previous session history
  lives implicitly in `completed[]` and `decisions[]`.
- **`decisions[]` is the most important section.** It is what survives `/compact`
  and session boundaries. Populate it aggressively.
- **`TASKS.md` should be git-tracked.** It is project state, not ephemeral tooling.
  Commit it alongside code changes.
