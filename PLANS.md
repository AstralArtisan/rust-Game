# PLANS.md — Task Handoff Template

<!-- 
  Usage: Claude fills in a new section below the template for each task.
  Codex reads this file as its execution spec.
  After Codex completes, Claude reviews the result against this plan.
  Completed plans can be moved to docs/plans/ for archival.
-->

---

## Template

```markdown
# Task: [short title]

## Why
[One paragraph: what problem this solves or what value it adds]

## Scope
[Bullet list of what IS and IS NOT in scope]

## Affected Files
[Exact file paths that will be created, modified, or deleted]

## Constraints
- [Technical constraints, e.g., "must work in both InGame and CoopGame"]
- [Style constraints, e.g., "data-driven via RON config"]
- [Compatibility constraints, e.g., "do not break existing save format"]

## Implementation Plan
1. [Step 1: concrete action]
2. [Step 2: concrete action]
3. ...

## Validation
```bash
cargo check --quiet
cargo test --quiet
# any additional commands
```
[Manual test steps if applicable]

## Risks
- [What could go wrong and how to mitigate]

## Codex Execution Brief
Read `AGENTS.md` for your execution contract.
Implement exactly the steps above. Run validation. Report results.
Do not expand scope beyond this plan.
```

---

## Current Task

(no active task)
