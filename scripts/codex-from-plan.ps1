# codex-from-plan.ps1 — Run Codex to implement the current plan.
# Usage: .\scripts\codex-from-plan.ps1
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Agents = Join-Path $RepoRoot "AGENTS.md"
$Plans  = Join-Path $RepoRoot "PLANS.md"

# Preflight checks
foreach ($f in @($Agents, $Plans)) {
    if (-not (Test-Path $f)) {
        Write-Error "ERROR: $f not found. Claude must write the plan first."
        exit 1
    }
}

if (-not (Get-Command codex -ErrorAction SilentlyContinue)) {
    Write-Error "ERROR: codex CLI not found. Install with: npm install -g @openai/codex"
    exit 1
}

Write-Host "=== Launching Codex from plan ==="
Write-Host "  AGENTS: $Agents"
Write-Host "  PLANS:  $Plans"
Write-Host ""

codex -a $Agents `
    "Read PLANS.md and implement the Current Task section exactly. Follow the execution contract in AGENTS.md. Keep scope tight - do not add anything not in the plan. Run validation commands from the plan. Report: files changed, commands run, test results, blockers, follow-ups."
