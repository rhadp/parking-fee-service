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

## Workflow Per Task

1. Create feature branch from `develop`: `git checkout -b feature/<task-name> develop`
2. Implement changes
3. Stage and commit with descriptive message: `git add . && git commit -m "<type>: <description>"`
4. Push and create PR via GitHub MCP server
5. Merge back to `develop` before starting next task

## Commit Message Format

Use conventional commits: `<type>: <description>`

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`

## Rules

- Never commit directly to `main` or `develop`
- One feature branch per task
- Merge `.gitignore` changes—never overwrite the file
- Use GitHub MCP server for all PR operations
