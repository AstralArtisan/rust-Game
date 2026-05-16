---
title: 勇闯方块城 — 增量修改方案（非重构）
date: 2026-05-15
status: approved
supersedes_workflow: 本轮由 Claude 直接在分支 little-refactor 实现，不走 plan-to-codex
based_on: docs/superpowers/specs/2026-04-29-full-refactor-implementation-plan.md (+ -design.md)
---

# 勇闯方块城 — 增量修改方案（非重构）

## Context

`2026-04-29-full-refactor-*.md` 是一份**从零重写**方案（方案 B：新仓库干净骨架 + 代码移植，升级 Bevy 0.18/0.15）。本项目**不做整仓重构**，而是在现有 `E:/rust_game_merge` 仓库内，**复用该 spec 的全部玩法设计**，以**就地增量修改**方式落地，同时更改 spec 中不合理的设计、修复游戏流程潜在 bug。

本文档是该转译后的完整记录与执行路线。执行优先级：**先修 bug + 不合理设计，再做内容扩充与架构迁移**。

工作方式：本轮由 **Claude 直接在分支 `little-refactor`** 实现（不走 Codex / plan-to-codex），每阶段自带影响文件与验证命令，按阶段提交。

关键决策（已确认 + 已验证）：
- **引擎保持 Bevy 0.14.2**，不升级 bevy / bevy_rapier2d 0.27 / lightyear 0.17.1。
- **状态机迁移到三层**（AppState → GamePhase → RoomPhase）——已验证 Bevy 0.14.2 原生支持 `SubStates`/`ComputedStates`（`bevy_state-0.14.2` prelude 导出，`src/state/sub_states.rs` 存在），与保持 0.14 不冲突，无需手写替代。
- **coop 完全重写**纳入范围（语义：拆 god 文件 + 成长链路并入单机共享路径，仍用 Lightyear 0.17.1，**不**做 netcode 从零重写）。
- 就地修改：**不**做 spec 的全仓 `src/` 目录树重排，只在某项改动确实需要时拆分文件。

## 一、spec 不合理设计 → 本计划的更改

| spec 设计 | 问题 | 本计划更改 |
|---|---|---|
| 升级 Bevy 0.18/0.15、rapier 0.33/0.29、Lightyear 0.26/0.19 | 跨大版本升级 = 全仓 API 重写，违背"不重构" | **撤销**，保持 0.14.2 / rapier 0.27 / lightyear 0.17.1 |
| 全新 `src/` 模块树（combat/、rooms/、enemy/behaviors/、skills/effects/ 全量重排） | 大规模文件搬迁 = 纯 churn + 巨大 diff + 回归风险 | **不做整体重排**；映射到现有结构（combat 在 `player/combat.rs`、特殊房在 `event_room/` `rewards/` `shop/`、`skills/` 已存在）。仅 coop 重写时拆 god 文件 |
| 在新仓库重写（方案 B） | 用户要求就地修改 | 全部改为现有仓库**就地 edit** |
| 设计文档 vs 实施计划数值矛盾 | 同一 spec 自相矛盾 | 统一裁定（见下），以"实施计划"为主、设计文档补充 |
| 事件房数量：design §3.4 写 14、design §6.1 列 17、impl plan 写 17 | design 内部矛盾 | 统一为 **17 种**（谜题 3 + 非战斗 10 + 战斗 4），`events.ron` 落地 |
| 终结技：spec 9 个全新，覆盖现有 4 槽（SwordArc/MarkedHunt/LightningDash/Relic） | 现有 `SkillSlots` 槽位基建可复用 | **保留槽位基建**，9 终结技做成 `skills.ron` + 槽位装配；不重写技能骨架 |
| coop "完全重写"字面=netcode 重写 | Lightyear 0.17.1 不升级，从零重写风险极高无收益 | 重定义为：(a) 拆 `runtime.rs`(2915行)/`ui.rs`(2749行)/`net.rs`(887行) 为 coop/ 子模块；(b) coop 成长并入单机 `AugmentInventory`/skills/progression 共享路径，删 `CoopRewardMode`/`RewardModifiers` 分叉；仍用 0.17.1 |
| 大量像素美术 / UI 精细化（spec Phase 9-10） | 用户优先级是 bug+设计 | 列为**末期延后阶段**，遵循"先功能后美术" |
| skills 消耗档位 design vs impl 略有出入 | 细节矛盾 | 以实施计划 §5.3 表为准，写入 `skills.ron` |

数值统一裁定：事件 **17** 种；强化 30 个 × **3 层**（含质变，现状 2 层）；终结技 **9** 个；怪物 **10** 种（现 9，新增 Lobber 投石者）；精英词缀 6；Boss 4；楼层 4 层、每层 **10** 房间（现 `game_balance.ron`=7，按 spec 调 10）。

## 二、已确认 Bug 清单（Phase 1 优先，含 file:line，均已读源核实）

1. **配置整体回退** `src/data/loaders.rs:~23` — `try_load_all()` 对 player/enemies/bosses/rewards/rooms/balance 用 `?`，**任一**必需 RON 失败 → 整个 registry 回退 `default_registry()`（默认值与现 RON 已分叉）。改为**按文件独立回退 + 显式定位是哪个文件**。
2. **存档严重缺失** `src/core/save.rs:36-59` — 仅存 floor/raw stats/RewardModifiers/cooldowns/achievements/enemy_spawn_count；**不存** `AugmentInventory`、`PlayerLevel`/XP、`SkillSlots`、楼层布局、`CurrentRoom`。F5/F9 丢失强化与等级。补齐并 bump `version`。
3. **事件房 Esc 重复触发** `src/gameplay/event_room/mod.rs:451-455` — Esc 仅置 `interaction_ready=true`，未置 `resolved`/未清 `event_type`，可重复触发同一事件。改为 Esc 不解决但**防重入**。
4. **奖励房 Back/Esc 软锁** `src/gameplay/rewards/systems.rs:398-413` — Back 重建圣所但 `flow.step` 仍 `Sanctuary`，可无限重选不结算、不发 `RoomClearedEvent`。修正 Back 语义，保唯一收敛出口。
5. **圣所空池软锁** `src/gameplay/rewards/systems.rs:567-581` — 无可升级且无觉醒项时 UI 仍显示"觉醒"但选择无效 → 卡死。空池给保底。
6. **商店无 Esc / 双购 / 楼层重置残留** `src/gameplay/shop/mod.rs:390-519`(无 Esc 退出)、`:518`(同帧双输入双扣)、`:83-102`(楼层重置不强制回 InGame)。加 Esc 退出、输入去抖、楼层重置状态收敛。
7. **progression/skills 不在 coop 运行** `src/gameplay/progression/mod.rs:26`、`src/gameplay/skills/mod.rs:38` — `.run_if(in_state(AppState::InGame))` 排除 CoopGame。**已重新定位到 Phase 5**：正确启用需 coop 主机权威门控（`is_coop_authority`），朴素放宽会让 coop client 跑单机楼层/升级并与 coop runtime 冲突 desync；Phase 2 仅做行为保持改写（保持单机门控），根治随 Phase 5 coop 成长统一一起做。
8. **RoomState 不是状态** `src/states.rs:28-35` — 普通 Resource，无 OnEnter/OnExit、Cleared 不锁门。Phase 2 迁 `RoomPhase` SubState 根治；Phase 1 先补关键 cleanup/锁门防重入。
9. **强化封顶 2 层** `src/gameplay/augment/data.rs:77` — `.min(2)`，spec 要 3 层质变。Phase 3 处理（列此备查）。

## 三、阶段路线图（bug 优先）

| 阶段 | 内容 | 性质 |
|---|---|---|
| **Phase 1** | Bug 1-8 修复 + 不合理设计纠正（撤销升级/目录重排、统一矛盾口径） | 修 bug，状态机无关项就地修 |
| **Phase 2**（已细化） | **仅 AppState→GamePhase 一层**：AppState 瘦身，覆盖层移入 `GamePhase`（manual SubStates，源 InGame\|CoopGame）。`in_state(AppState::InGame)` 网关做**行为保持式**改写为 `in_state(AppState::InGame).and_then(in_state(GamePhase::Playing))`，使覆盖层正确暂停玩法且语义与现状完全等价。**RoomState 保持 Resource 不变**；**不动 coop CoopPhase** | 基础架构迁移 |
| **Phase 2b（延后）** | RoomState→RoomPhase 房间流程语义重设计（同帧写后读、BossFight 变体、coop 主机权威），可并入 Phase 5 | 语义重设计 |
| **Phase 3** | 单机内容对齐 spec：强化 2→3 层+质变；9 终结技；事件房→17；圣所对称三选一；商店三区；+Lobber 与 charger/bomber/shielder 重做；Boss 削弱项；掉落/经济；XP 曲线 | 内容扩充 |
| **Phase 4** | 难度缩放、NG+ 5 级+难度档、存档 UI、成就/图鉴/设置界面 | 内容扩充 |
| **Phase 5** | **coop 完全重写**：拆 god 文件为 coop/ 子模块；成长并入单机共享路径（删 CoopRewardMode）；合作缩放；事件房在 coop 保留 | 架构重写（就地） |
| **Phase 6** | PVP 模块化（net/arena/visuals/ui 拆分），规则保持轻量原型 | 小幅 |
| **Phase 7（延后）** | 像素美术、UI 精细化、视觉特效 | 末期，先功能后美术 |

风险缓解：Phase 1 只选**状态机无关** bug 就地修；#8（RoomState）延后 Phase 2b；#7（progression/skills coop）重新定位 Phase 5（需 coop 主机权威门控）。

## 四、各系统修改规格（现状 → 目标 → 改法 → 影响文件）

### 4.1 三层状态机（Phase 2，已细化为仅第一层）
现状：扁平 `AppState`(19 变体，`src/states.rs:4-26`)+`RoomState`(Resource，4 变体含 BossFight)。

**Phase 2 实际范围**：只做 AppState→GamePhase。
- `AppState` 留：Loading, MainMenu, InGame, MultiplayerMenu, CoopMenu, CoopLobby, CoopGame, PvpMenu, PvpLobby, PvpGame, PvpResult。
- `GamePhase`：**manual `impl SubStates`**（`type SourceStates = Option<AppState>`，`should_exist` 对 `Some(InGame)|Some(CoopGame)` 返回 `Some(Playing)`，否则 `None`），变体 = 现有覆盖层同名 `{Playing(default), Paused, RewardSelect, AugmentSelect, LevelUpSelect, Shop, EventRoom, GameOver, Victory}`（沿用原名以最小化迁移；`BossChest` 等 Phase 3 内容到时再加，避免空壳变体）。
- 注册：`app.add_sub_state::<GamePhase>()`（manual impl + `States` + `FreelyMutableState`）。
- 迁移要点：`in_state(AppState::Shop)` → `in_state(GamePhase::Shop)`；写覆盖层的系统 `NextState<AppState>`→`NextState<GamePhase>`；**"从覆盖层返回游戏"语义** `set(AppState::InGame)` → `set(GamePhase::Playing)`，而"从菜单进入游戏" `set(AppState::InGame)` 保持不变——需按每处意图判断，非纯 find-replace。
- progression/skills 与其余玩法 `in_state(AppState::InGame)` 一律做**行为保持式**改写 `in_state(AppState::InGame).and_then(in_state(GamePhase::Playing))`（仍单机门控，覆盖层正确暂停玩法，coop 不变）。Bug#7（progression/skills 进 coop）**不在 Phase 2**，重新定位 Phase 5。
- 含 PvpGame/menu 的复合 run 条件只对 InGame 分支加 `.and_then(GamePhase::Playing)`（PvpGame 下无 GamePhase）；`core/input.rs` 输入采集是基础设施，覆盖层也需读输入，**不门控**。

**不在 Phase 2**：`RoomState` 保持 Resource 原样（语义重设计风险，见 Phase 2b）；coop `CoopPhase` 不动，coop 下 GamePhase 维持 `Playing`，coop 模态仍走 CoopPhase（统一留 Phase 5）。

影响：`src/states.rs`、`src/app.rs`、`src/gameplay/mod.rs`、所有 `in_state/OnEnter/OnExit` 覆盖层引用、`src/core/save.rs`、`src/ui/*`、`src/gameplay/{progression,skills,event_room,rewards,shop,...}`。RoomState 相关引用与 coop CoopPhase 不动。

### 4.2 伤害管线（Phase 3）
现状散在 `player/combat.rs`、`enemy/systems.rs`、`enemy/boss.rs`。目标 spec §3.3 五阶段 Event 链（immunity→shield→modifier→apply→post）。改法：现有 combat 内抽 System 链（不新建 combat/ 目录），强化/Boss 修正以组件标记参与 modifier，去 if-else 硬编码。

### 4.3 强化系统 30×3 层（Phase 3）
现状 30 个、`AugmentInventory`、封顶 2(`augment/data.rs:77`)。目标 3 层质变（spec §4 全表）。改法：`.min(2)`→`.min(3)`；`augments.ron` 扩每项 3 层 `params`；effects 增 stacks==3 质变分支；数值入 RON。

### 4.4 能量与终结技 9 个（Phase 3）
现状 4 槽 SwordArc/MarkedHunt/LightningDash/Relic。目标 9 终结技（近战/远程/辅助各 3），3 档能量/CD（spec §5.3），槽位 1-4 按楼层解锁，强化联动。改法：保留 `SkillSlots`/`PlayerSkillState`；新增 `skills.ron`；`skills/` 下 melee/ranged/support 效果系统；Boss 宝箱/圣所/商店给终结技选择。

### 4.5 事件房 17 种（Phase 3）
现状 10 类，`event_room/mod.rs`。目标 谜题 3 + 非战斗 10 + 战斗 4。改法：新建/扩 `events.ron`；先修 Bug#3，补齐类型与结算，每类唯一清房收敛。

### 4.6 奖励房圣所（Phase 3）
现状非对称 Heal→条件 Upgrade/Awakening→Revelation。目标对称三选一（疗愈/强化锻造/启示，spec §6.2）。改法：修 Bug#4/#5；一次性三卡；启示=+1 级+升级选择+50 金；锻造空池保底传说。

### 4.7 商店房（Phase 3）
现状 lines/augment_lines/utility_lines 键 1-8。目标 属性区 4 / 强化区(普80/精150/传250+升级120+终结技180) / 工具区(治疗R/能量F/护身符)；刷新首免后 30 递增 15。改法：修 Bug#6；按 spec §6.3 入新建 `shop.ron`。

### 4.8 怪物/精英/Boss（Phase 3）
现状 9 怪无 Lobber、6 词缀、4 Boss。目标 +Lobber；Charger/Bomber/Shielder 加强；精英 3 层后固定 2 词缀；MirrorWarden 削弱（分身 HP20%/频率降）。改法：`enemies.ron`+Lobber、新建 `elite_affixes.ron`、`boss.ron` 调参；增量加 Lobber/重做 charger 移动。数值入 RON。

### 4.9 地图与房间流程（Phase 3）
现状 `game_balance.ron` 楼层 4/每层 7。目标 每层 10 房（1 战斗/2-9 选门/10 Boss），门规则（≥1 战斗、奖励每层≤1、商店事件不连续、精英从第 3 房起），尺寸递增。改法：调 `game_balance.ron`/`rooms.ron`，门规则补全 + 类型着色。

### 4.10 掉落/经济/成长（Phase 3-4）
目标 金币 普3-6/精12-20/Boss30-50、每层 100-180；XP 曲线；难度缩放；NG+ 5 级+难度档。改法：数值入 `balance.ron`/`ng_plus.ron`(新建)，`progression/` 增 difficulty/ng_plus。

### 4.11 存档（Phase 1 修 + Phase 4 完善）
Phase 1 补 AugmentInventory/PlayerLevel/XP/SkillSlots；Phase 4 补楼层布局/房间 ID/事件房状态，读档从房间初始态加载（spec §8.4）。

### 4.12 coop 完全重写（Phase 5）
现状 god 文件 + `CoopPhase`/`CoopRewardMode`/`RewardModifiers` 独立成长，零 AugmentInventory（`coop/runtime.rs:105` vs 单机 `rewards/systems.rs:85`）。目标 拆 `coop/{protocol,lobby,sync,authority,visuals,scaling,ui}.rs`；成长走 Phase 2 后共享 `GamePhase` 路径复用 AugmentInventory/skills/progression，删 CoopRewardMode；合作缩放 怪 HP×1.6/伤害×1.2/数量+30%；事件房在 coop 保留。约束：仍 Lightyear 0.17.1，主机权威不变，先 LocalDebug 联调。

### 4.13 PVP（Phase 6）
`pvp/` 按 spec §9.2 拆 net/arena/visuals/ui，规则保持 3 命无技能轻量原型，预留扩展。

### 4.14 UI（Phase 3 随功能 + Phase 7 精细化）
功能期：HUD（HP/能量/金币/楼层/终结技槽 CD）、升级三栏、暂停+角色面板、Boss 宝箱、商店三区、圣所三卡、事件房——随系统就地改 `src/ui/*`。Phase 7：像素风统一 + `/ui-ux-pro-max` + `/game-assets`。

## 五、配置文件改动

新建：`skills.ron`、`events.ron`、`shop.ron`、`ng_plus.ron`、`elite_affixes.ron`、`balance.ron`（或并入 `game_balance.ron`）。
扩充：`augments.ron`(3 层)、`enemies.ron`(+Lobber)、`boss.ron`(MirrorWarden 削弱)、`game_balance.ron`(4×10 房)。
原则：数值不硬编码进 Rust；配合 Bug#1 修复后单文件错误只回退该文件。

## 六、验证方案

- 每阶段：`cargo check --quiet`（音频 dead_code 已知，不新增）、`cargo test --quiet`（基线 45 通过，不回退）。
- 功能（手动 `cargo run`，原生 Bevy 无浏览器 MCP，逐项核对）：
  1. 状态切换无崩溃/软锁（重点 Bug#3-6 路径）
  2. 存档 F5/F9 恢复强化+等级（Bug#2）
  3. 单坏 RON 只回退该文件（Bug#1）
  4. 强化 3 层质变、9 终结技、事件 17、圣所三选一、商店三区
  5. coop 本地联调（CLAUDE.md 的 `LOCAL_NET_DEBUG`），两人成长一致、事件房可玩、缩放生效
- 阶段产出后执行 `doc-maintenance` + `git-maintenance`。

## 七、执行与交接

- 分支：`little-refactor`（从 `claude-playground` 创建），承接现工作树未提交改动。
- 由 Claude 直接实现（不走 Codex），按 Phase 顺序推进，每 Phase 自带影响文件 + 验证命令，阶段里程碑提交（用户确认后）。
- 文档：本文件为长期记录；每阶段后按 `doc-maintenance` 更新 `docs/`、`CLAUDE.md`，按 `git-maintenance` 提交。
