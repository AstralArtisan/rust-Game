# Codex 交接文档 — little-refactor 剩余工作

> 本文件是 Codex 的权威任务规范（见 `AGENTS.md`：Codex 写代码前必须完整阅读 PLANS.md）。Claude 已亲自完成 Phase 1+2 并提交；Phase 2b/3-7 由用户操作 Codex 实施。

## 0. 给 Codex 的执行须知（最先读）

- 工作分支：**`little-refactor`**（从 `claude-playground` 派生）。不要切分支、不要 push（push 由用户决定）。
- 权威设计（**必读**）：`docs/superpowers/specs/2026-05-15-incremental-modification-plan.md` —— 完整玩法设计、数值、分阶段路线图。本交接文档是其「当前状态 + 执行指令」视图，深度细节以该 spec 为准。
- 迭代历史：`docs/05_iteration_history.md`（Phase 1/2 条目，记录已做改动与关键决策）。
- 历史参考 `docs/superpowers/specs/2026-04-29-full-refactor-*.md` 是「从零重写」旧方案——**仅供玩法设计参考，不要据其重写/升级引擎/重排目录**。
- 执行契约：严格遵守 `AGENTS.md`（范围、最小 diff、阻塞即停并报告、报告格式）。
- 引擎锁定：**Bevy 0.14.2 / bevy_rapier2d 0.27 / lightyear 0.17.1**，不得升级。
- 数据驱动：所有数值进 `assets/configs/*.ron`，**禁止**在 Rust 硬编码平衡数值。

## 1. 仓库现状（Phase 1+2 已完成）

分支 `little-refactor`，提交链：
- `0b20f499` Phase 1 基线 + bug 修复（承接 rune/curse→AugmentInventory 工作树）
- `62f8f5ce` Phase 1 文档
- `45b30b0d` Phase 2 状态机分层（**当前 HEAD**）

**验证基线（Codex 必须维持，不得回退）**：
- `cargo check --quiet` 通过，**仅** 3 个既有 `src/core/audio.rs` dead-code 警告；不得引入新警告。
- `cargo test --quiet`：**45/45 通过**；不得减少或破坏。

已完成内容摘要：
- **Phase 1 bug 修复**：配置按文件独立回退（`data/loaders.rs`）；存档补 `AugmentInventory/PlayerLevel/SkillSlots`、`version=2`、旧档 `#[serde(default)]` 兼容（`core/save.rs`）；事件房 Esc 防重 roll（`event_room/mod.rs`）；奖励房 Back 不再 RNG 重抽 + 空池安全收敛（`rewards/systems.rs`）；商店加 Esc 退出（`shop/mod.rs`）。
- **Phase 2 状态机两层化**：见第 2 节不变量。

工作树未跟踪项（**勿动、勿提交**，非本工作范围）：`assets/generated/`、`assets/textures/{backgrounds,bosses,enemies}/`、`prompt.md`、`.superpowers/`、`docs/repository_status_report_2026-04-28.md`、`docs/superpowers/specs/2026-04-29-*.md`。

## 2. 当前架构关键不变量（Codex 不得破坏）

1. **两层状态机**（`src/states.rs`）：
   - `AppState`（顶层，11 态）：`Loading/MainMenu/InGame/MultiplayerMenu/CoopMenu/CoopLobby/CoopGame/PvpMenu/PvpLobby/PvpGame/PvpResult`。
   - `GamePhase`（**manual `impl SubStates`**，`SourceStates=Option<AppState>`，存在于 `InGame|CoopGame`，默认 `Playing`）：`Playing/Paused/RewardSelect/AugmentSelect/LevelUpSelect/Shop/EventRoom/GameOver/Victory`。
   - `RoomState` **仍是 Resource**（非状态机），4 变体 `Idle/Locked/Cleared/BossFight` 不变。
2. **玩法系统门控范式**：`.run_if(in_state(AppState::InGame).and_then(in_state(GamePhase::Playing)))`。新增任何玩法系统必须照此门控（覆盖层自动暂停玩法）。覆盖层 UI/逻辑用 `GamePhase::X`（OnEnter/OnExit/in_state/`NextState<GamePhase>`）。
3. **状态转移语义**：从覆盖层回游戏 = `NextState<GamePhase>::set(GamePhase::Playing)`；进入/退出整局 = `NextState<AppState>`。`AugmentChoices`/`LevelUpChoices.return_state` 类型是 `Option<GamePhase>`。
4. **含 PvpGame 的复合 run 条件**只对 `InGame` 分支加 `.and_then(in_state(GamePhase::Playing))`（PvpGame 下 `GamePhase` 不存在）。
5. **`core/input.rs` 输入采集不门控**（覆盖层也需读输入）。
6. **coop 边界**：`src/coop/` 的 `CoopPhase` 与网络栈在 Phase 5 之前**不得改动**；**禁止**在无 `is_coop_authority` 门控的情况下把 `ProgressionPlugin`/`SkillsPlugin` 放进 `CoopGame`（会致 coop client desync —— 这是 Bug#7，归 Phase 5）。
7. `src/gameplay/session_core/` 是单机/coop 共享规则层；新增共享规则放这里，勿重复。
8. 所有局内实体打 `InGameEntity` 标记以便状态切换清理。

## 3. 剩余工作路线图（Codex 按序执行；每阶段独立交接、独立验证、阶段末提交不 push）

### Phase 2b（默认跳过，除非用户明确指定）：`RoomState` → `RoomPhase` SubState
语义重设计，风险高：现 `RoomState` 是可同帧写后读的 Resource，被 33+ 处 `*room_state = ...` 直接赋值；`BossFight` 变体与 `Entering/Active/Cleared/Exiting` 不对应；且与 coop 主机权威耦合。**不要主动做**；可并入 Phase 5。

### Phase 3：单机内容对齐 spec（核心，工程量最大）
逐项执行，数值/详表见 `2026-05-15-...-plan.md` §4-§8 与 2026-04-29 设计 spec 对应章节（Codex 必读对应章节再动手）：
1. **强化 2→3 层质变**：`gameplay/augment/data.rs` `add()` 的 `.min(2)`→`.min(3)`；`assets/configs/augments.ron` 每个强化扩为 3 层 `params`（Lv1/Lv2/Lv3 质变）；`augment/effects.rs` 各效果增加 `stacks==3` 质变分支。30 个强化完整表见 spec §4.4。
2. **9 终结技**：保留现有 `SkillSlots`/`PlayerSkillState` 槽位基建（**不重写技能骨架**）；新建 `assets/configs/skills.ron`（能量/CD/档位见 spec §5.3）；`gameplay/skills/` 下按 melee/ranged/support 加效果系统；Boss 宝箱/圣所/商店提供终结技选择；槽位 1-4 按楼层解锁。
3. **事件房 17 种**：新建/扩 `assets/configs/events.ron`（谜题 3 + 非战斗 10 + 战斗 4，见 spec §6.1）；补齐类型与结算分支，保证每类有唯一清房收敛（Phase 1 已修 Esc 重 roll，勿回退该修复）。
4. **奖励房圣所对称三选一**：疗愈/强化锻造/启示一次性三卡（spec §6.2）；用「无可升级项则给传说强化」替换 Phase 1 的空池安全收敛占位。
5. **商店三区**：属性区 4 / 强化区（普 80/精 150/传 250 + 升级 120 + 终结技 180）/ 工具区；价格入新建 `assets/configs/shop.ron`；刷新首免后 30 递增 15（spec §6.3）。
6. **怪物/精英/Boss**：新增 Lobber 投石者；Charger/Bomber/Shielder 加强；精英 3 层后固定双词缀；MirrorWarden 削弱（分身 HP 20%）。数值入 `enemies.ron`、新建 `elite_affixes.ron`、`boss.ron`（spec §7）。
7. **地图**：每层 10 房（1 战斗/2-9 选门/10 Boss），门生成规则（≥1 战斗、奖励每层≤1、商店事件不连续、精英从第 3 房起），尺寸递增；调 `game_balance.ron`（现 `floor_rooms` 7→10）、`rooms.ron`（spec §8）。
8. **掉落/经济/XP**：金币 普 3-6/精 12-20/Boss 30-50、每层 100-180；XP 曲线；数值入 `balance.ron`/`game_balance.ron`（spec §6.4）。

### Phase 4：成长系
难度楼层缩放；NG+ 5 级解锁 + 难度档（新建 `ng_plus.ron`，spec §8.5）；存档补楼层布局/当前房间（读档从房间初始态）；成就/图鉴/设置界面。

### Phase 5：coop 完全重写
拆 god 文件 `coop/runtime.rs`(2915)/`ui.rs`(2749)/`net.rs`(887) 为 spec §9.1 的 `coop/{protocol,lobby,sync,authority,visuals,scaling,ui}.rs`；coop 成长并入 Phase 2 后的共享 `GamePhase` 路径（复用 `AugmentInventory`/skills/progression），删除 `CoopRewardMode`/`RewardModifiers` 分叉；**根治 Bug#7**（progression/skills 配 `is_coop_authority` 门控进 coop）；合作数值缩放（怪 HP×1.6/伤害×1.2/数量+30%）；事件房在 coop 保留。仍用 lightyear 0.17.1，主机权威不变，先 LocalDebug 联调。

### Phase 6：PVP 模块化
`src/pvp/` 拆 `net/arena/visuals/ui`，规则保持 3 命无技能轻量原型。

### Phase 7（延后到功能完成后）：像素美术 + UI 精细化 + 视觉特效。

## 4. 意图与方向（Codex 必须理解的总纲）

- **复用** 2026-04-29 全面重构 spec 的**全部玩法设计**，但以**「就地增量修改现有仓库」**落地，**不是**从零重写、不升级引擎、不做无关的大规模目录/文件重排。
- 优先级：先修 bug 与不合理设计（Phase 1 已做）→ 状态机分层（Phase 2 已做）→ 单机内容（Phase 3）→ 成长系（Phase 4）→ coop 重写（Phase 5）→ PVP（Phase 6）→ 美术（Phase 7）。**先功能后美术**。
- 最小 diff、保留现有公共 API/组件签名/系统调度（除非该阶段 spec 明确授权改动）。
- 单机与 coop 共用游戏逻辑，差异走网络同步层与 `session_core`，不在玩法代码里散布 `if coop`。

## 5. 验证与报告（按 AGENTS.md）

每阶段至少：
```
cargo check --quiet   # 不得新增警告（基线：3 个既有 audio dead-code）
cargo test --quiet    # 不得回退（基线：45 passed）
```
原生 Bevy 无法浏览器自动化；手感/运行时行为类改动需在报告中明确「需用户 `cargo run` 实测」并列出实测清单。完成后按 `AGENTS.md` 的报告格式输出（Files Changed / Commands Run / Test Results / Blockers / Follow-ups）。阶段里程碑由用户决定提交/推送时机。

## 6. 已知判断与坑（Codex 注意，勿推翻）

- **Bug#7**（progression/skills 不在 coop 运行）：只能在 **Phase 5** 配 `is_coop_authority` 门控解决；Phase 3/4 **不要**把这两个 plugin 放进 `CoopGame`。
- 商店「同帧双购 / 楼层重置 UI 残留」经核实在当前代码**不可达**——**不要**加防御性死代码。
- `RoomState` 在 Phase 2b 之前保持 Resource，**不要**自行改成状态机。
- Phase 1 的事件房 Esc 防重 roll、奖励房 Back 不重抽/空池收敛是有意修复，Phase 3 在其基础上扩展，**不要回退**。
- 引入新覆盖层（如 spec 的 `BossChest`）时：加进 `GamePhase` 枚举并在 `app.rs` 之外按需注册系统；当前未加是为避免空壳变体，Phase 3 实现 Boss 宝箱时再加。

## 7. 关键文件索引（Codex 快速定位）

| 区域 | 文件 |
|---|---|
| 状态机 | `src/states.rs`、`src/app.rs` |
| 强化 | `src/gameplay/augment/{data,effects}.rs`、`assets/configs/augments.ron` |
| 终结技 | `src/gameplay/skills/`、`src/gameplay/player/components.rs`（SkillSlots）、新建 `skills.ron` |
| 事件房 | `src/gameplay/event_room/mod.rs`、新建 `events.ron` |
| 奖励房 | `src/gameplay/rewards/systems.rs`、`src/ui/reward_select.rs` |
| 商店 | `src/gameplay/shop/mod.rs`、`src/ui/shop.rs`、新建 `shop.ron` |
| 怪物/Boss | `src/gameplay/enemy/{systems,boss,components}.rs`、`enemies.ron`/`boss.ron`/新建 `elite_affixes.ron` |
| 地图 | `src/gameplay/map/`、`game_balance.ron`/`rooms.ron` |
| 成长/NG+ | `src/gameplay/progression/`、新建 `ng_plus.ron` |
| 存档 | `src/core/save.rs` |
| coop | `src/coop/`（Phase 5 前只读不改） |
| 共享规则 | `src/gameplay/session_core/mod.rs` |
