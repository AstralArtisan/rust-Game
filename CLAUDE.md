# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo run                   # Development run
cargo run --release         # Release build
cargo check                 # Compile check
cargo test                  # Run unit tests (24 tests)
```

### Local Multiplayer Debugging (PowerShell)

**Coop (Lightyear, UDP 3457):**
```powershell
# Host
$env:LOCAL_NET_DEBUG="1"; $env:LOCAL_NET_DEBUG_MODE="coop"; $env:LOCAL_NET_DEBUG_ROLE="host"; cargo run

# Client
$env:LOCAL_NET_DEBUG="1"; $env:LOCAL_NET_DEBUG_MODE="coop"; $env:LOCAL_NET_DEBUG_ROLE="client"; $env:LOCAL_NET_DEBUG_HOST="127.0.0.1"; cargo run
```

**PVP (Custom UDP 3456):**
```powershell
# Host
$env:LOCAL_NET_DEBUG="1"; $env:LOCAL_NET_DEBUG_MODE="pvp"; $env:LOCAL_NET_DEBUG_ROLE="host"; cargo run

# Client
$env:LOCAL_NET_DEBUG="1"; $env:LOCAL_NET_DEBUG_MODE="pvp"; $env:LOCAL_NET_DEBUG_ROLE="client"; $env:LOCAL_NET_DEBUG_HOST="127.0.0.1"; cargo run
```

**In-game keybinds:** `F5` saves to `saves/run_save.ron`, `F9` loads it.

## Architecture

**勇闯方块城** is a 2D top-down roguelike built with Bevy 0.14 (ECS), bevy_rapier2d for physics, and two separate network stacks: Lightyear 0.17 (Coop) and a custom UDP implementation (PVP).

The entire game is a **single Bevy app** with one `AppState` enum covering all modes. `GamePlugin` in `src/app.rs` is the central assembler.

### Layer Stack

```
src/main.rs          → App setup, window config
src/app.rs           → GamePlugin (mounts all sub-plugins)
src/states.rs        → AppState + RoomState enums

src/core/            → Infrastructure: assets, input, audio, camera, save, achievements, local_debug
src/data/            → Config: RON file loaders → GameDataRegistry resource
src/gameplay/        → Shared game logic (used by singleplayer AND Coop)
src/coop/            → Lightyear-based host-authority Coop network layer
src/pvp/             → Custom UDP PVP network layer
src/ui/              → All menus, HUD, pause, notifications
src/utils/           → Math, RNG, easing, collision, entity helpers
```

### State Machine

`Loading → MainMenu → InGame ↔ RewardSelect / Shop / Paused → GameOver/Victory`
`MainMenu → CoopMenu → CoopLobby → CoopGame`
`MainMenu → PvpMenu  → PvpLobby  → PvpGame → PvpResult`

**RoomState** (sub-state inside a run): `Idle → Locked (combat/puzzle active) → Cleared`, with `BossFight` as a special phase.

### Key Design Decisions

- **`src/gameplay/session_core/`** contains shared rules (reward curves, shop logic, room completion, death judgment) deliberately reused by both singleplayer and Coop — do not duplicate this logic elsewhere.
- **Config-driven gameplay**: enemy stats, boss phases, rewards, room generation, and balance are all loaded from `assets/configs/*.ron`. Modify those files, not hardcoded constants, to tune gameplay.
- **Puzzles** (`src/gameplay/puzzle/`) only run in `AppState::InGame` (singleplayer). They are not replicated to Coop.
- **Coop uses host authority**: `src/coop/runtime.rs` runs all simulation on the host; clients send inputs and receive state. This is the most complex file in the repo.
- **`InGameEntity` marker** (`src/utils/entity.rs`) is added to all entities that should be despawned on state transitions.

### Complexity Hot Spots

- `src/coop/runtime.rs` — Host authority simulation loop
- `src/coop/ui.rs` — Replicated entity visualization
- `src/gameplay/enemy/systems.rs` — AI behaviors and spawning
- `src/gameplay/session_core/mod.rs` — Shared game rules
- `src/ui/hud.rs` — HUD update systems

### Config Files (`assets/configs/`)

| File | Controls |
|------|----------|
| `player.ron` | HP, speed, dash, energy, cooldowns |
| `enemies.ron` | Stats per enemy type (melee_chaser, ranged_shooter, charger, flanker, sniper, support_caster) |
| `boss.ron` | Boss phase parameters by floor |
| `rewards.ron` | Reward text, stat modifiers, drop rates |
| `rooms.ron` | Room generation parameters |
| `game_balance.ron` | Global difficulty, floor count, room counts |

## Documentation

Detailed documentation lives in `docs/`:
- `02_architecture.md` — State machine, ECS layers, system ordering
- `03_module_design.md` — Per-module responsibilities and dependencies
- `06_multiplayer_and_risks.md` — Coop/PVP architecture and known technical debt
- `07_extension_guide.md` — How to add new enemies, rewards, rooms, or modes
