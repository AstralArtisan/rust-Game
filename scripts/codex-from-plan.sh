#!/usr/bin/env bash
# codex-from-plan.sh — Run Codex to implement the current plan.
# Usage: ./scripts/codex-from-plan.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
AGENTS="$REPO_ROOT/AGENTS.md"
PLANS="$REPO_ROOT/PLANS.md"

# Preflight checks
for f in "$AGENTS" "$PLANS"; do
  if [ ! -f "$f" ]; then
    echo "ERROR: $f not found. Claude must write the plan first." >&2
    exit 1
  fi
done

if ! command -v codex &>/dev/null; then
  echo "ERROR: codex CLI not found. Install with: npm install -g @openai/codex" >&2
  exit 1
fi

echo "=== Launching Codex from plan ==="
echo "  AGENTS: $AGENTS"
echo "  PLANS:  $PLANS"
echo ""

codex -a "$AGENTS" \
  "Read PLANS.md and implement the Current Task section exactly. \
Follow the execution contract in AGENTS.md. \
Keep scope tight — do not add anything not in the plan. \
Run validation commands from the plan. \
Report: files changed, commands run, test results, blockers, follow-ups."
