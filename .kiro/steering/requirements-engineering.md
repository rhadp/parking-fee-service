---
inclusion: always
---

# Requirements Engineering Guidelines

Rules for writing requirements, design, and task documents in `.kiro/specs/`.

## Component Naming

Use `SCREAMING_SNAKE_CASE` for all system components (PARKING_APP, LOCKING_SERVICE, DATA_BROKER). This distinguishes components from general terms.

## Requirements Document (`requirements.md`)

### Structure

1. **Introduction** - Component purpose and system context
2. **Glossary** - Domain terms, acronyms, component definitions
3. **Requirements** - Numbered requirements with user stories and acceptance criteria

### Requirement Format

```markdown
### Requirement N: [Name]

**User Story:** As a [role], I want [capability], so that [benefit].

#### Acceptance Criteria

1. WHEN [condition] THEN [component] SHALL [behavior]
2. IF [condition] THEN [component] SHALL [behavior]
3. THE [component] SHALL [behavior]
```

### Acceptance Criteria Rules

| Keyword | Usage |
|---------|-------|
| `SHALL` | Mandatory behavior (always use) |
| `WHEN/THEN` | Event-driven behavior |
| `IF/THEN` | Conditional behavior |
| `THE` | Unconditional requirement |

Always include:
- Specific signal names, endpoints, interfaces
- Measurable values (timeouts, retry counts, thresholds)

## Tasks Document (`tasks.md`)

### Task Syntax

```markdown
- [ ] N. Parent task
  - [ ] N.1 Subtask
    - Implementation detail
    - _Requirements: X.Y, X.Z_
```

### Checkbox States

| Syntax | Meaning |
|--------|---------|
| `- [ ]` | Not started (required) |
| `- [ ]*` | Not started (optional) |
| `- [x]` | Completed |
| `- [-]` | In progress |
| `- [~]` | Queued |

Tasks are **required by default**. Mark optional tasks with `*` after checkbox: `- [ ]* Optional task`

### Task Organization

- Group by: setup → core logic → testing → integration
- Add checkpoints after major milestones
- Reference requirements: `_Requirements: X.Y_`

### Test Task Annotations

- Unit/integration tests: `**Validates: Requirements X.Y**`
- Property-based tests: `**Property N: [Name]**` (references design doc properties)

## Traceability

Maintain bidirectional links:
- Acceptance criteria → Tasks (via requirement numbers)
- Property tests → Design correctness properties
- Use glossary terms consistently across all documents
