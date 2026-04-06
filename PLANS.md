# Phase 4c: 事件房（合并 Puzzle → Event）

## Context

Phase 4a（掉落物）和 4b（商店扩展）已完成。Phase 4c 将 `RoomType::Puzzle` 重命名为 `RoomType::Event`，保留原有 3 种谜题作为事件子类型，新增 8 种事件，共 11 种。新增 `AppState::EventRoom` 用于非战斗事件的 UI 交互。

## Current Task

### 目标

1. `RoomType::Puzzle` → `RoomType::Event`（全局替换）
2. 新增 `EventType` 枚举（11 种事件）
3. 新增 `AppState::EventRoom` 状态
4. 新建 `src/gameplay/event_room/mod.rs`（事件房逻辑）
5. 新建 `src/ui/event_room.rs`（事件房 UI）
6. 修改所有引用 `RoomType::Puzzle` 的文件

### Affected files

**新建：**
- `src/gameplay/event_room/mod.rs`
- `src/ui/event_room.rs`

**修改：**
- `src/gameplay/map/room.rs`
- `src/states.rs`
- `src/gameplay/mod.rs`
- `src/app.rs`
- `src/gameplay/puzzle/mod.rs`
- `src/gameplay/session_core/mod.rs`
- `src/gameplay/enemy/systems.rs`
- `src/gameplay/map/generator.rs`
- `src/gameplay/map/tiles.rs`
- `src/gameplay/map/doors.rs`
- `src/ui/hud.rs`
- `src/ui/mod.rs`
- `src/coop/runtime.rs`
- `src/coop/ui.rs`
- `src/core/achievements.rs`

### 详细改动

#### 1. `src/gameplay/map/room.rs` — 重命名枚举变体

```rust
pub enum RoomType {
    Start,
    Normal,
    Shop,
    Reward,
    Event,   // 原 Puzzle
    Boss,
}
```

#### 2. `src/states.rs` — 新增 AppState

在 `AppState` 枚举中添加 `EventRoom`（放在 `Shop` 后面）：
```rust
    Shop,
    EventRoom,
    GameOver,
```

#### 3. 全局替换 `RoomType::Puzzle` → `RoomType::Event`

以下文件中所有 `RoomType::Puzzle` 替换为 `RoomType::Event`，`Puzzle` 相关的显示文本保持为"事件"（大部分已经是）：

| 文件 | 改动 |
|------|------|
| `src/gameplay/map/generator.rs:316` | `RoomType::Puzzle => 2` → `RoomType::Event => 2`（权重保持 2） |
| `src/gameplay/map/generator.rs:261-265` | Coop 中 `RoomType::Puzzle` → `RoomType::Event`（仍转为 Normal） |
| `src/gameplay/map/tiles.rs:160` | `RoomType::Puzzle` → `RoomType::Event`，标签改为 `"事件房"` |
| `src/gameplay/map/doors.rs:196` | `RoomType::Puzzle` → `RoomType::Event`，标签保持 `"事件"` |
| `src/gameplay/enemy/systems.rs:261,306` | `RoomType::Puzzle` → `RoomType::Event` |
| `src/gameplay/session_core/mod.rs` | `RoomType::Puzzle` → `RoomType::Event` |
| `src/ui/hud.rs:819,1087,1173` | `RoomType::Puzzle` → `RoomType::Event`，标签保持 `"事件"` |
| `src/coop/runtime.rs:60-72` | `RoomType::Puzzle` → `RoomType::Event`（normalize 函数） |
| `src/coop/runtime.rs:2826-2855` | 测试中 `Puzzle` → `Event` |
| `src/coop/ui.rs:2528` | `RoomType::Puzzle` → `RoomType::Event` |
| `src/core/achievements.rs:159` | `RoomType::Puzzle` → `RoomType::Event`，成就 ID 保持 `PuzzleSolver` |

#### 4. 新建 `src/gameplay/event_room/mod.rs`（~300 行）

```rust
use bevy::prelude::*;
use crate::gameplay::augment::data::{AugmentId, AugmentInventory};
use crate::gameplay::curse::{CurseId, CurseState};
use crate::gameplay::map::room::{CurrentRoom, RoomId};
use crate::gameplay::map::InGameEntity;
use crate::gameplay::player::components::{Gold, Health, Player};
use crate::gameplay::puzzle;
use crate::core::events::RoomClearedEvent;
use crate::states::AppState;
use crate::utils::rng::GameRng;

pub struct EventRoomPlugin;

impl Plugin for EventRoomPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveEvent>()
            .add_systems(
                Update,
                select_and_spawn_event
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                Update,
                event_room_input
                    .run_if(in_state(AppState::EventRoom)),
            );
    }
}
```

**EventType 枚举：**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    // 旧谜题
    PressurePlate,
    SwitchOrder,
    TrapSurvival,
    // 新事件
    Gambler,          // 50g → 随机强化
    CurseAltar,       // 接受诅咒 → 精英强化
    BloodPact,        // -30% HP → 2选1强化
    Treasure,         // 免费 Common 强化 + 30g
    HealingSpring,    // 回复 40% max HP
    Merchant,         // 2 个半价强化
    TimedChallenge,   // 30s 击杀 → 精英强化（战斗事件）
    EliteEncounter,   // 单挑精英 → 精英强化（战斗事件）
}
```

**ActiveEvent 资源：**
```rust
#[derive(Resource, Default)]
pub struct ActiveEvent {
    pub event_type: Option<EventType>,
    pub resolved: bool,
    pub room: Option<RoomId>,
    pub choices: Vec<EventChoice>,  // 用于需要选择的事件
}

pub struct EventChoice {
    pub label: String,
    pub description: String,
}
```

**系统：**

1. `select_and_spawn_event` — 当进入 Event 房且 `ActiveEvent` 未设置时触发：
   - 谜题类型（PressurePlate/SwitchOrder/TrapSurvival）：调用现有 `puzzle::spawn_puzzle_for_room()`，不转状态
   - 战斗类型（TimedChallenge/EliteEncounter）：设 `RoomState::Locked`，生成敌人，不转状态
   - 非战斗事件（其余 6 种）：设置 `ActiveEvent` 的 choices，转到 `AppState::EventRoom`

2. `event_room_input` — 在 `AppState::EventRoom` 中处理玩家选择：
   - 按 1/2 选择选项，按 Esc 离开（放弃事件）
   - 选择后应用效果：
     - Gambler：扣 50g，随机 `AugmentInventory::add(random_augment)`
     - CurseAltar：`CurseState::add_curse(random_curse)`，然后给精英强化
     - BloodPact：`hp.current *= 0.7`，给 2 选 1 强化
     - Treasure：直接给 Common 强化 + 30g
     - HealingSpring：`hp.current = (hp.current + hp.max * 0.4).min(hp.max)`
     - Merchant：展示 2 个半价强化可购买（用 Gold）
   - 完成后发 `RoomClearedEvent`，回到 `AppState::InGame`

**事件选择随机权重：**
- 谜题类型（3种）：各权重 1（总 3）
- 非战斗事件（6种）：各权重 2（总 12）
- 战斗事件（2种）：各权重 1（总 2）
- 总权重 17，谜题约 18%，非战斗约 70%，战斗约 12%

#### 5. 修改 `src/gameplay/enemy/systems.rs`

在 Event 房的 spawn 逻辑中（原 ~L306）：
```rust
RoomType::Event => {
    // 事件房的 spawn 由 event_room 模块处理
    // 谜题类型和战斗类型会在 select_and_spawn_event 中设置 RoomState::Locked
    // 非战斗类型不锁门
}
```

不再直接调用 `spawn_puzzle_for_room`，改由 `event_room` 模块统一调度。

#### 6. 修改 `src/gameplay/puzzle/mod.rs`

- `PuzzlePlugin` 保留，系统保留
- `spawn_puzzle_for_room()` 改为 `pub` 供 `event_room` 调用
- 不再由 `enemy/systems.rs` 直接调用

#### 7. 新建 `src/ui/event_room.rs`（~100 行）

```rust
use bevy::prelude::*;
use crate::core::assets::GameAssets;
use crate::gameplay::event_room::ActiveEvent;
use crate::states::AppState;
use crate::ui::widgets;

#[derive(Component)]
pub struct EventRoomUi;

pub fn setup_event_room_ui(mut commands: Commands, assets: Res<GameAssets>, event: Res<ActiveEvent>) {
    // 全屏半透明背景 + 居中面板
    // 显示事件标题 + 描述 + 选项列表（按 1/2 选择，Esc 离开）
}

pub fn cleanup_event_room_ui(mut commands: Commands, q: Query<Entity, With<EventRoomUi>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}
```

#### 8. 修改 `src/ui/mod.rs`

添加 `pub mod event_room;`

#### 9. 修改 `src/gameplay/mod.rs`

添加 `pub mod event_room;` 和注册 `event_room::EventRoomPlugin`

#### 10. 修改 `src/app.rs`

注册 EventRoom 状态的 OnEnter/OnExit：
```rust
.add_systems(OnEnter(AppState::EventRoom), ui::event_room::setup_event_room_ui)
.add_systems(OnExit(AppState::EventRoom), ui::event_room::cleanup_event_room_ui)
```

#### 11. 修改 `src/gameplay/session_core/mod.rs`

Event 房通关不触发 RewardSelect（奖励由事件自身处理）。

### 需要注意的导入

- `event_room/mod.rs` 需要：`AugmentId`, `AugmentInventory`, `CurseId`, `CurseState`, `Gold`, `Health`, `Player`, `GameRng`, `RoomClearedEvent`, `AppState`, `InGameEntity`
- 从 `data::registry::GameDataRegistry` 获取强化池
- 从 `gameplay::rewards::systems` 中复用强化选择逻辑（如果 `generate_augment_choices` 是 pub 的话），否则自行实现简单版本

### 验证命令
```bash
cargo check --quiet
cargo test --quiet
```
