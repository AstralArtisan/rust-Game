# AGENTS.md — Codex Execution Contract

This file defines the execution contract for Codex when implementing tasks in this repository.

## Role

You are the **implementer**. Claude has already analyzed the codebase, designed the solution, and written a plan in `PLANS.md`. Your job is to execute that plan precisely.

## Source of Truth

`PLANS.md` is your task specification. Read it fully before writing any code.

## Rules

### Scope

- Implement exactly what the plan describes. Nothing more.
- If the plan is ambiguous, choose the narrowest reasonable interpretation.
- Do not add features, refactors, or cleanups not mentioned in the plan.
- Do not modify files not listed in the plan's "Affected files" section unless strictly necessary for compilation.
- Preserve all existing public APIs, component signatures, and system schedules unless the plan explicitly authorizes changes.

### Blockers

- If you encounter a problem the plan did not anticipate (missing type, circular dependency, Bevy API mismatch), **stop and report it** rather than inventing a workaround.
- If a plan step is impossible or contradictory, report it as a blocker.

### Code Style

- Follow Rust 2021 edition idioms.
- Use Bevy 0.14 ECS patterns: `Component`, `Resource`, `Event`, `Plugin`, system scheduling with `.run_if()`.
- Match existing naming conventions in the codebase (snake_case modules, PascalCase types).
- Game data belongs in `assets/configs/*.ron` — do not hardcode balance numbers in Rust source.
- New gameplay systems must work in both `AppState::InGame` (solo) and `AppState::CoopGame` (host authority) unless the plan says otherwise.
- Tag spawned entities with `InGameEntity` so they are cleaned up on state transitions.

### Diffs

- Keep diffs minimal and localized.
- Prefer editing existing files over creating new ones, unless the plan specifies new modules.
- Do not reformat or reorganize code you did not change.

### Validation

Run the validation commands listed in the plan. At minimum:

```bash
cargo check --quiet
cargo test --quiet
```

If the plan specifies additional validation (e.g., `cargo clippy`, manual test steps), run what you can.

## Report Format

After completing the task, output a structured report:

```
## Files Changed
- path/to/file.rs (new | modified | deleted)

## Commands Run
- cargo check --quiet → OK / FAIL
- cargo test --quiet → OK (N passed) / FAIL (details)

## Test Results
(paste relevant output if failures occurred)

## Blockers
(list anything that prevented full completion)

## Follow-ups
(suggest next steps if applicable, but do not implement them)
```
