# P4inz — Claude Code Instructions

Read `AGENTS.md` and `ROADMAP.md` before making changes.

## Non-negotiable rules

- Preserve the locked architecture.
- Prioritize correctness, security, safety, reliability, and maintainability over speed.
- Do not introduce paid infrastructure as a required dependency.
- Do not expose or commit secrets.
- Do not use destructive commands without explicit authorization.
- Do not modify architecture casually.
- Do not add dependencies without a clear technical reason.
- Run formatting, linting, and relevant tests after changes.
- Never automatically push to GitHub.
- Keep changes focused and reviewable.
- If implementation conflicts with the architecture, stop and report the conflict instead of silently redesigning it.

## Git workflow

Inspect → Implement → Test → Audit → Review → Commit.

Never skip validation for convenience.
