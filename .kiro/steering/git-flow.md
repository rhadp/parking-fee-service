---
inclusion: always
---

# Git Workflow

This project uses gitflow-workflow for version control.

## Branch Structure

| Branch | Purpose |
|--------|---------|
| `main` | Production-ready code |
| `develop` | Integration branch for features |
| `feature/*` | Individual feature/task work |

## Commit Message Format

Use conventional commits: `<type>: <description>`

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`

## Rules

- Never commit directly to `main` or `develop`
- Before starting to work on a task, create a new feature branch
- One feature branch per task
- Merge `.gitignore` changes — NEVER overwrite the file
- Use the GitHub MCP server for all git operations
