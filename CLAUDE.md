# CLAUDE.md

本文件为 Claude Code (claude.ai/code) 在此仓库中工作提供指导。

## 常用命令

```bash
cargo run                   # 开发运行
cargo run --release         # 发布构建
cargo check                 # 编译检查
cargo test                  # 运行单元测试（24 个）
```

### 本地多人联调（PowerShell）

**合作模式 Coop（Lightyear，UDP 3457）：**
```powershell
# 主机端
$env:LOCAL_NET_DEBUG="1"; $env:LOCAL_NET_DEBUG_MODE="coop"; $env:LOCAL_NET_DEBUG_ROLE="host"; cargo run

# 客户端
$env:LOCAL_NET_DEBUG="1"; $env:LOCAL_NET_DEBUG_MODE="coop"; $env:LOCAL_NET_DEBUG_ROLE="client"; $env:LOCAL_NET_DEBUG_HOST="127.0.0.1"; cargo run
```

**对战模式 PVP（自定义 UDP 3456）：**
```powershell
# 主机端
$env:LOCAL_NET_DEBUG="1"; $env:LOCAL_NET_DEBUG_MODE="pvp"; $env:LOCAL_NET_DEBUG_ROLE="host"; cargo run

# 客户端
$env:LOCAL_NET_DEBUG="1"; $env:LOCAL_NET_DEBUG_MODE="pvp"; $env:LOCAL_NET_DEBUG_ROLE="client"; $env:LOCAL_NET_DEBUG_HOST="127.0.0.1"; cargo run
```

**游戏内快捷键：** `F5` 保存到 `saves/run_save.ron`，`F9` 读取存档。

## 架构

**勇闯方块城** 是一款基于 Bevy 0.14（ECS）的 2D 俯视角 Roguelike，使用 bevy_rapier2d 处理物理，并有两套独立的网络栈：Lightyear 0.17（Coop）和自定义 UDP 实现（PVP）。

整个游戏是一个**单一 Bevy App**，用一个 `AppState` 枚举覆盖所有模式。`src/app.rs` 中的 `GamePlugin` 是核心装配点。

### 模块层级

```
src/main.rs          → App 创建、窗口配置
src/app.rs           → GamePlugin（挂载所有子插件）
src/states.rs        → AppState + RoomState 枚举

src/core/            → 基础设施：资源、输入、音频、相机、存档、成就、本地联调
src/data/            → 配置：RON 文件加载器 → GameDataRegistry 资源
src/gameplay/        → 共享游戏逻辑（单机与 Coop 均使用）
src/coop/            → 基于 Lightyear 的主机权威 Coop 网络层
src/pvp/             → 自定义 UDP PVP 网络层
src/ui/              → 所有菜单、HUD、暂停、通知
src/utils/           → 数学、RNG、缓动、碰撞、实体工具
```

### 状态机

`Loading → MainMenu → InGame ↔ RewardSelect / Shop / Paused → GameOver/Victory`
`MainMenu → CoopMenu → CoopLobby → CoopGame`
`MainMenu → PvpMenu  → PvpLobby  → PvpGame → PvpResult`

**RoomState**（单局内部子状态）：`Idle → Locked（战斗/解谜进行中）→ Cleared`，`BossFight` 为特殊阶段。

### 关键设计决策

- **`src/gameplay/session_core/`** 包含单机与 Coop 共用的规则（奖励曲线、商店逻辑、房间通关、死亡判定）——不要在其他地方重复这些逻辑。
- **配置驱动的游戏玩法**：敌人数值、Boss 阶段、奖励、房间生成和平衡参数均从 `assets/configs/*.ron` 加载。调整数值请修改这些文件，而非硬编码常量。
- **解谜系统**（`src/gameplay/puzzle/`）仅在 `AppState::InGame`（单机）中运行，不会复制到 Coop。
- **Coop 采用主机权威**：`src/coop/runtime.rs` 在主机端运行所有模拟；客户端发送输入并接收状态。这是仓库中最复杂的文件。
- **`InGameEntity` 标记**（`src/utils/entity.rs`）添加到所有需要在状态切换时被销毁的实体上。

### 关键实现细节

1. **共享逻辑与网络专用逻辑**：`gameplay/` 目录包含在单机和 Coop 中均运行的核心系统。在 Coop 模式下，这些系统仅在主机端执行（通过 `is_coop_authority` 和 `in_state(AppState::CoopGame)` 标记）。客户端主要负责输入复制和复制实体的可视化表现。

2. **本地调试系统**：`src/core/local_debug.rs` 中的 `LocalDebugPlugin` 无需真实网络即可进行本地多人测试。它会自动将窗口并排放置，并为调试会话提供带后缀的独立存档文件。

3. **存档系统**：使用 `ron` 格式保存可读存档。存档数据包括版本号、楼层、玩家属性、成就和敌人刷新计数。`PendingLoad` 资源确保存档只在切换到 `InGame` 状态时才被应用。

4. **网络栈分离**：Coop 使用 Lightyear 0.17.1 实现带房间推进和实体复制的主机权威多人模式；PVP 使用轻量级自定义 UDP 协议实现直接玩家对战，状态同步更简单。

### 复杂度热点

- `src/coop/runtime.rs` — 主机权威模拟循环与会话管理
- `src/coop/ui.rs` — 复制实体可视化与会话状态 UI
- `src/gameplay/enemy/systems.rs` — 多种敌人类型的复杂 AI 行为
- `src/gameplay/session_core/mod.rs` — 集中式游戏规则与进程逻辑
- `src/ui/hud.rs` — 多游戏状态下的动态 HUD 更新

### 配置文件（`assets/configs/`）

| 文件 | 控制内容 |
|------|----------|
| `player.ron` | 血量、速度、冲刺、能量、冷却时间 |
| `enemies.ron` | 各敌人类型数值（melee_chaser、ranged_shooter、charger、flanker、sniper、support_caster） |
| `boss.ron` | 各楼层 Boss 阶段参数 |
| `rewards.ron` | 奖励文本、属性修正、掉落率 |
| `rooms.ron` | 房间生成参数 |
| `game_balance.ron` | 全局难度、楼层数、房间数 |

## 开发指南

### 添加新内容

1. **新敌人**：添加到 `enemies.ron`，在 `src/gameplay/combat/` 创建组件，并在 `src/gameplay/enemy/systems.rs` 中注册
2. **新奖励**：在 `rewards.ron` 中定义，在 `src/gameplay/rewards/` 中实现逻辑，并与 `session_core` 集成
3. **新房间类型**：更新 `rooms.ron` 生成参数，并在 `src/gameplay/map/` 中添加对应逻辑

### 网络开发

- **Coop**：游戏逻辑的修改必须同时兼容单机和主机权威模拟。先用本地调试模式测试。
- **PVP**：独立网络栈，追求简洁。玩家间直接通信，状态复制最小化。

### 质量说明

- 当前实现存在编译警告（未使用代码、弃用 API），已在项目文档中记录
- 24 个单元测试覆盖核心游戏系统
- 主执行二进制名为 `block_city_adventure`
- 窗口标题为"勇闯方块城"
