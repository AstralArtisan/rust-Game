# CLAUDE.md

本文件为 Claude Code (claude.ai/code) 在此仓库中工作提供指导。

@AGENTS.md

## 常用命令

```bash
cargo run                   # 开发运行
cargo run --release         # 发布构建
cargo check                 # 编译检查
cargo test                  # 运行单元测试（44 个）
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

## 项目维护

### 文档维护

使用skill： `doc-maintenance`。项目的所有工程文档在 `docs/` 中维护。你可以通过 `docs/00_index.md` 来获取文档索引，以了解项目信息。在后续的工作中，你需要持续维护这些文档，记录每一次修改的内容与迭代的目的、想法等，并根据项目情况更新 `README.md` 、 `CLAUDE.md`和 `docs/` 中对应的文档。在工作时，若发现一些不带编号的文档已经彻底过时，你可以直接将其删除。

### Github维护

使用skill：`git-maintenance`。该项目的仓库位于 `https://github.com/AstralArtisan/rust-Game/`。当前以分支 `saved_version` 为基础进行改进，工作分支为 `claude-playground`。每次修改后，及时提交commit，当取得重大突破时，及时push。当测试顺利通过、符合全部预期后，可以合并进main。当前的main仍以单机模式为主。

### 编码规范

按照rust语言程序设计规范进行编码，注意变量生命周期的维护等问题，合理使用bevy等组件进行项目搭建。

### 项目预期

首先实现联机合作、联机对战功能的正常运行，然后对单机模式进行修改，包括玩法、成长、怪物设计、数值等，并能够同步到联机合作模式中。

## Plan-to-Codex 工作流

本仓库使用 Claude 规划 + Codex 执行的分工模式。所有代码交给codex完成，你不进行代码的修改。如果调用失败，你不要进行代码的修改工作。

### 角色分工

| 角色   | 职责                                                        |
| ------ | ----------------------------------------------------------- |
| Claude | 分析仓库、理解需求、编写计划（`PLANS.md`）、审查 Codex 产出 |
| Codex  | 按计划实现代码、保持最小 diff、运行验证、报告结果           |

### 工作流程

1. **Claude 规划**：使用 `plan-to-codex` skill，分析任务后写入 `PLANS.md`
2. **Codex 执行**：使用 `codex:rescue` skill 启动 Codex（见下方调用方式）
3. **Claude 审查**：对比 `PLANS.md` 与实际改动，检查范围、质量、回归
4. **收尾**：执行 `doc-maintenance` 和 `git-maintenance` skills

### 关键文件

| 文件        | 用途                                         |
| ----------- | -------------------------------------------- |
| `AGENTS.md` | Codex 的执行契约（范围、代码风格、报告格式） |
| `PLANS.md`  | 任务交接模板，Claude 写入，Codex 读取        |

### Codex 调用方式（必须遵守）

**主要方式**：使用 `codex:rescue` skill，传入以下 prompt：

```
读取项目根目录的 PLANS.md，严格按照计划实现所有代码改动。遵守 AGENTS.md 执行契约。完成后运行 cargo check --quiet 和 cargo test --quiet，并按 AGENTS.md 格式输出报告。
```

- `codex:rescue` 会自动处理线程续接（`--resume`/`--fresh`）
- 如果是全新任务，选 `Start a new Codex thread`
- 如果是跟进修复，选 `Continue current Codex thread`

**备选方式**（`codex:rescue` 失败时）：直接调用 `mcp__codex-cli__codex` MCP 工具：
- `sandbox`: `"workspace-write"`（必须，否则无法写文件）
- `workingDirectory`: `"E:/rust_game_merge"`
- `fullAuto`: `true`

**禁止的方式：**
- 不要在 Bash 中运行 `codex` CLI（需要 TTY，会失败）
- 不要调用 `node codex-companion.mjs`（需要 TTY）

### 默认行为

Claude 在此仓库中默认为规划者和审查者。除非用户明确要求"直接实现"或"自己动手"，否则 Claude 应：

- 先检查再规划
- 优先写计划而非写代码
- 定义精确的影响范围和验证命令
- 将实现委托给 Codex

### 计划同步（必须执行）

**每次 `ExitPlanMode` 被用户批准后，必须立即执行以下操作：**

1. 读取批准的计划文件（路径显示在 ExitPlanMode 返回信息中，位于 `C:\Users\OMEN\.claude\plans\*.md`）
2. 将其完整内容写入项目根目录的 `PLANS.md`
3. 告知用户："计划已同步到 `PLANS.md`，可以运行 Codex。"

这确保 Codex 运行 `./scripts/codex-from-plan.ps1` 时能读取到最新计划。

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
src/gameplay/player/ → 玩家组件、战斗、冲刺、连击、技能系统
src/gameplay/rune/   → 铭文系统：槽位、等级、装备管理
src/gameplay/curse/  → 诅咒系统：生命周期、效果修正
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
- **战斗蓄力系统**：能量不自然回复，通过战斗行为充能（近战命中+8、远程+4、击杀+12等），蓄满后按 1/2/3 释放终结技（剑气斩/标记猎杀/闪电冲刺）。配置在 `player.ron` 的 `charge_*` 字段。
- **铭文系统**（`src/gameplay/rune/`）：双轨奖励的第二轨道。4 个槽位（近战/远程/冲刺/终结技），每槽装 1 个铭文（替换而非叠加）。铭文改变能力行为而非纯数值加成。配置在 `runes.ron`。
- **诅咒系统**（`src/gameplay/curse/`）：祝福祠堂选择铭文时附带的临时负面效果，持续 N 个房间后自动消除。有诅咒时不会出现新的祝福房。配置在 `curses.ron`。
- **祝福祠堂**：原 Reward 房在 Floor 2+ 变为祝福祠堂，展示 2 个铭文+诅咒选项。复用 `AppState::RewardSelect` 的 Blessing 模式。
- **技能槽位**：数字键 1-4 对应 HUD 底部技能栏。`SkillSlots` 组件记录解锁状态，`PlayerSkillState` 管理释放中的技能状态。
- **事件房交互**（`src/gameplay/event_room/mod.rs`）：仿商店模式，进入事件房只显示交互提示，按 E 才激活事件。`init_event_for_room` 选事件+设标记，`event_interact_system` 按 E 激活。Esc 不解决事件，允许重新交互。
- **Boss 传送门**（`src/gameplay/rewards/systems.rs`）：Boss 通关后 AugmentSelect → 返回 InGame → 地图中心生成 `BossPortal`，玩家按 E 推进楼层。不自动推进，给玩家收集掉落物的时间。
- **升级回血选择**（`src/ui/levelup_select.rs`）：升级 UI 双栏布局——左栏固定回血（按1），右栏三个随机属性强化（按2/3/4）。回血量由 `heal_amount` 函数基于楼层和最大生命值计算。
- **小怪血条**（`src/gameplay/enemy/systems.rs`）：`EnemyHealthBar` + `EnemyHealthBarFill` 世界空间 Sprite，跟随敌人位置，颜色随血量变化（绿→黄→红），Boss 排除。
- **精英房独立逻辑**：`RoomType::Elite` 独立分支，`spawn_elite_room_enemies` 固定 1 精英（1.4x 体积）+ 2 普通，通关 100% AugmentSelect。

### 关键实现细节

1. **共享逻辑与网络专用逻辑**：`gameplay/` 目录包含在单机和 Coop 中均运行的核心系统。在 Coop 模式下，这些系统仅在主机端执行（通过 `is_coop_authority` 和 `in_state(AppState::CoopGame)` 标记）。客户端主要负责输入复制和复制实体的可视化表现。

2. **本地调试系统**：`src/core/local_debug.rs` 中的 `LocalDebugPlugin` 无需真实网络即可进行本地多人测试。它会自动将窗口并排放置，并为调试会话提供带后缀的独立存档文件。

3. **存档系统**：使用 `ron` 格式保存可读存档（当前 `version: 2`）。存档数据包括版本号、楼层、玩家属性、`RewardModifiers`、冷却、成就、敌人刷新计数，以及 `AugmentInventory`/`PlayerLevel`/`SkillSlots`（little-refactor Phase 1 起补齐，旧版 1 档案经 `#[serde(default)]` 仍可读）。楼层布局/当前房间尚未持久化，详见 `docs/superpowers/specs/2026-05-15-incremental-modification-plan.md`。`PendingLoad` 资源确保存档只在切换到 `InGame` 状态时才被应用。

4. **网络栈分离**：Coop 使用 Lightyear 0.17.1 实现带房间推进和实体复制的主机权威多人模式；PVP 使用轻量级自定义 UDP 协议实现直接玩家对战，状态同步更简单。
- **程序化音效系统**（`src/core/audio.rs`）：启动时用波形合成生成 13 种 WAV 音效，插入 `Assets<AudioSource>`。`SfxEvent` 事件驱动播放，桥接系统自动将 `DamageAppliedEvent`/`RoomClearedEvent`/`BossPhaseChangeEvent` 转换为音效。配置在 `audio.ron`。
- **打击暂停系统**（`src/gameplay/effects/hitstop.rs`）：通过 `Time<Virtual>` 时间缩放实现，命中/暴击/击杀时短暂冻结画面增强打击感。
- **屏幕闪光系统**（`src/gameplay/effects/screen_flash.rs`）：全屏 UI 覆盖层，`ease_out_expo` 快速衰减 alpha。Boss 死亡和阶段切换时触发。
- **BGM 状态机**（`src/core/audio.rs` `BgmState`）：根据 `AppState`/`RoomState` 自动切换曲目类型（Menu/Exploration/Combat/Boss），预留外部音频加载接口。

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
| `runes.ron` | 铭文定义（30个铭文的槽位、等级、名称、描述、取舍、价格） |
| `curses.ron` | 诅咒定义（5种诅咒的名称、效果、持续时间） |
| `rooms.ron` | 房间生成参数 |
| `game_balance.ron` | 全局难度、楼层数、房间数 |
| `audio.ron` | 主音量、音效音量、BGM 音量、pitch 随机变化幅度 |
| `effects.ron` | 粒子数、打击暂停时长、屏幕闪光时长、血条 lerp 速度 |

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
- 44 个单元测试覆盖核心游戏系统
- 主执行二进制名为 `block_city_adventure`
- 窗口标题为"勇闯方块城"

