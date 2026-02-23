# Agent Instructions

Instructions for coding agents (Cursor, Claude Code, Codex, etc.) working on
this repository. Treat this file as mandatory policy for every coding session.

## Understand Before You Code (MANDATORY)

Before making any changes, orient yourself:

1. **Read project documentation:** `README.md`, `prd.md` (or `.specs/prd.md`)
   for high-level requirements, and any specifications in `.specs/`.
2. **Read architecture decision records** in `docs/adr/`, if any exist.
3. **Read agent memory:** `docs/memory.md`, if it exists — accumulated
   knowledge from prior coding sessions.
4. **Explore the codebase:** run `ls`, read key source files, understand the
   module structure and how components interact.
5. **Check git state:** `git log --oneline -20`, `git status --short --branch`.
6. **Run existing tests** to confirm the baseline is green. If tests fail,
   fix them before starting new work.

**Important:** Read all documents and code in depth, understand how the system works. Don't skim.

**Important:** Only read files tracked by git. Skip anything matched by
`.gitignore`. When in doubt, run `git ls-files` to see what's tracked.

Do not implement anything before completing these steps.

## Scope Discipline

- Focus on one coherent change per session.
- Do not include unrelated "while here" fixes.
- If asked for multiple changes, complete one and hand off the rest.
- Priority: fix broken behavior before adding new behavior.

## Git Workflow

- **Branch policy:** never commit directly to `main` or `develop`. Create a
  feature branch per change: `feature/<descriptive-name>`.
- **Conventional commits:** use `<type>: <description>` (e.g. `feat:`, `fix:`,
  `refactor:`, `docs:`, `test:`, `chore:`).
- **Commit discipline:** only commit files relevant to the current change. Keep
  commits focused and traceable.
- **Never add `Co-Authored-By` lines.** No AI attribution in commits — ever.
- **Landing:** push the feature branch to `origin` and confirm a clean working
  tree before ending the session.

## Quality Gates

Before committing, run all quality checks relevant to the files you changed:

- Tests (unit, integration, e2e as applicable)
- Linters and formatters
- Build / type-check

Fix failures before proceeding. No regressions allowed.

## Documentation

- **ADRs** live in `docs/adr/{decision}.md`.
- **Other docs** live in `docs/{topic}.md`.
- When you add or change user-facing behavior, public APIs, configuration, or
  architecture, update the relevant documentation in the same session.

## Session Completion

A session is not complete until:

1. All quality gates pass.
2. Changes are committed with a clear conventional commit message.
3. The feature branch is pushed to `origin`.
4. `git status` shows a clean working tree.
5. You provide a brief handoff note summarizing what was done and what remains.
