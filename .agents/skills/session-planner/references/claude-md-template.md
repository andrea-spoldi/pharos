# CLAUDE.md Lean Template

This is the canonical template for a lean `CLAUDE.md`.
Target: **under 50 lines**. Every line is overhead on every turn.

The file must contain only things that are:

1. **Non-obvious** — Claude would not infer this from the code.
1. **Always true** — not session-specific, not stale history.
1. **Consequential** — getting this wrong would cause real problems.

-----

## Template

```markdown
# [Project Name]

## Security — read first
<!-- Rules that prevent costly or irreversible mistakes. Always keep these. -->
- Never commit `.env` files or any file containing credentials.
- Never log authentication tokens, API keys, or PII.
- Production configs live in [Vault / SSM / wherever] — not in this repo.

## Architecture constraints
<!-- Non-obvious structural rules. If Claude would likely violate these without being told, list them. -->
- [Example: The event bus is single-threaded — never await inside an event handler.]
- [Example: kubescope-core and kubescope-ui are separate crates. No UI code in core.]
- [Example: All DB access goes through the repository layer — never query inline in handlers.]

## Intentional decisions
<!-- Counterintuitive choices that look like bugs but aren't. -->
- [Example: We use polling here, not webhooks — the external API does not support webhooks.]
- [Example: Error messages are intentionally vague on auth failures — do not make them specific.]

## Stack shorthand
<!-- One-liners that save Claude from inferring the stack. Keep to 3-5 lines max. -->
- Language: [Go 1.24 / Rust 1.78 / TypeScript 5.x]
- Key deps: [Cobra, kube-rs, GPUI, etc.]
- Test runner: [go test / cargo test / vitest]
- Lint: [golangci-lint / clippy / eslint]
```

-----

## What to delete

These are safe to remove. Claude already does them by default:

```markdown
# DELETE THESE — they cost tokens and change nothing

- Write clean, readable code
- Add comments to explain complex logic
- Follow the existing code style in the file
- Use meaningful variable names
- Don't leave debug statements in production code
- Write production-quality code
- You are an expert senior software engineer with X years of experience...
- Think step by step before responding
- Always consider edge cases
```

-----

## Staleness signals — delete when these apply

```markdown
# DELETE THESE when they no longer apply

- "We migrated from X to Y in [past date], you may see old X patterns"
  → Delete after the migration is complete and old code is gone.

- "The auth module was rewritten after [event] — avoid /legacy/"
  → Delete once /legacy/ is removed.

- "We're currently evaluating X vs Y"
  → Delete once the decision is made. Move outcome to "Intentional decisions".

- "[Person] owns this module, coordinate with them before changing"
  → Delete once the team changes or the constraint no longer applies.
```

-----

## Audit checklist (run periodically or on session start)

- [ ] Is the file under 50 lines?
- [ ] Does every line pass the "non-obvious + always-true + consequential" test?
- [ ] Are security rules still accurate and at the top?
- [ ] Are there any stale migration notes or past-tense history sections?
- [ ] Is the stack shorthand still accurate (Go version, key dep versions)?
- [ ] Are there any "you are an expert" or default-behaviour instructions to delete?
