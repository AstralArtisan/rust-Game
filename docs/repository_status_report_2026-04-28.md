# 仓库状态报告

审查日期：2026-04-28  
审查对象：`E:\rust_game_merge` 当前工作树  
当前分支：`claude-playground`  
当前 HEAD：`753db41c docs: 记录 Phase 10 迭代历史（组长例会准备+架构审查+文档更新）`

## 1. 摘要结论

当前仓库是一个基于 Bevy 0.14 的 2D 俯视角动作 roguelike 项目，包名为 `block_city_adventure`。代码已经形成了相对完整的单机闯关循环：主菜单进入单机、程序化楼层、普通/精英/商店/奖励/事件/Boss 房间、近战/远程/冲刺/能量技能、9 类小怪、4 个 Boss 原型、掉落物、金币、XP、升级、30 个强化、圣所奖励、事件房、商店、Boss 传送门、胜利/失败流程。

多人部分包含两套架构：

- 合作闯关使用 Lightyear `0.17.1`，是主机权威的同步架构，Host 模拟游戏，Client 上传输入并渲染 replicated 实体。
- PVP 使用自写 UDP 协议，是独立的 2P 对战原型，规则和单机/合作闯关基本分离。

当前最重要的状态判断：

- `cargo check --quiet` 通过，但有 `src/core/audio.rs` 的 dead code 警告。
- `cargo test --quiet` 通过，45 个测试全部通过。
- 工作树不是干净状态。当前报告基于未提交工作树，而不是纯 HEAD。
- 旧的 rune/curse 系统在当前工作树中已经从 Rust 源码和配置删除，但部分文档仍把它描述为当前系统。
- 单机玩法比文档中的部分旧描述更靠前；合作模式则与最新单机成长系统存在明显分叉。

## 2. 审查范围与依据

本次审查覆盖：

- Rust 源码：`src/` 下 107 个 Rust 文件。
- 游戏配置：`assets/configs/*.ron`。
- 资源状态：`assets/textures/`、未跟踪的生成资源目录。
- 项目文档：`README.md`、`CLAUDE.md`、`docs/*.md`、`PLANS.md`。
- git 历史：最近提交、阶段性提交、标签。
- 构建验证：`cargo check --quiet`、`cargo test --quiet`。

主要验证结果：

```text
cargo check --quiet -> OK, with warnings
cargo test --quiet  -> OK, 45 passed
```

`cargo check` 与 `cargo test` 都报告同一类警告：`SfxHandles` 中多个字段、`SfxHandles::get`、`BgmTrack` 的若干变体未被使用。这说明当前仓库不再是历史文档中描述的“零警告”状态。

## 3. 当前工作树状态

当前 `git status --short` 显示大量未提交变更。重点如下：

- 修改了核心代码：
  - `src/app.rs`
  - `src/coop/runtime.rs`
  - `src/core/assets.rs`
  - `src/data/definitions.rs`
  - `src/data/loaders.rs`
  - `src/gameplay/enemy/systems.rs`
  - `src/gameplay/enemy/boss.rs`
  - `src/gameplay/event_room/mod.rs`
  - `src/gameplay/map/generator.rs`
  - `src/gameplay/map/tiles.rs`
  - `src/gameplay/rewards/systems.rs`
  - `src/gameplay/session_core/mod.rs`
  - `src/ui/reward_select.rs`
  - 以及其他 UI、掉落、玩家、配置相关文件。

- 删除了旧系统文件：
  - `assets/configs/runes.ron`
  - `assets/configs/curses.ron`
  - `src/gameplay/rune/data.rs`
  - `src/gameplay/rune/mod.rs`
  - `src/gameplay/curse/mod.rs`

- 新增但未跟踪的资源：
  - `assets/generated/`
  - `assets/textures/backgrounds/`
  - `assets/textures/bosses/`
  - `assets/textures/enemies/`
  - `prompt.md`

`git diff --stat` 显示当前工作树约 39 个文件变更，约 1236 行新增、2531 行删除。最大的变化集中在：

- 删除 rune/curse 旧链路。
- 重写或大幅收缩 `session_core` 和 `reward_select`。
- 将 `AugmentPlugin` 收敛进 `GameplayPlugin`。
- 将事件房 UI 纳入 `UiPlugin`。
- 接入部分纹理资源与 `use_sprite_textures` 配置。

这意味着当前仓库处于一个“已可编译测试通过，但有较大未提交重构”的中间状态。

## 4. Git 历史概览

最近历史显示项目按阶段推进：

- 早期：建立 Bevy 单机骨架，加入房间、玩家、敌人、基础战斗。
- 合并期：引入合作模式和 PVP 原型。
- Phase 4：加入掉落物、扩展商店、事件房。
- Phase 5：加入 Bomber、Shielder、Summoner 三种敌人，加入精英词缀，强化 TideHunter。
- Phase 6：清理警告，加入精英词缀标签，外部化奖励数值。
- Phase 7：修复若干游戏体验问题。
- Phase 8：清理 UI、重设计事件房流程、修复进门位置。
- Phase 9：事件房重构、Boss 传送门、掉落物平衡、升级回血、精英房重设计、小怪血条、词缀标签修复。
- Phase 10：主要是文档、会议汇报、架构审查。
- 当前未提交工作：删除旧 rune/curse、进一步收敛奖励/强化架构、接入部分美术资源。

当前标签包括：

- `pre-xp-refactor`
- `pre-augment-system`
- `saved-version-20260330-161713`

这些标签说明 XP 和强化系统是后续较大的架构转向点。

## 5. 项目技术栈与顶层架构

`Cargo.toml` 当前关键依赖：

- `bevy = 0.14`
- `bevy_rapier2d = 0.27`
- `bevy_kira_audio = 0.20`
- `lightyear = 0.17.1`
- `serde`
- `ron`
- `bincode`
- `rand`
- `anyhow`
- `thiserror`

Rust edition 当前为 2024。

入口结构：

- `src/main.rs` 创建 Bevy `App`，设置窗口标题为“勇闯方块城”，注册 `GamePlugin`。
- `src/app.rs` 注册核心插件、数据插件、玩法插件、合作插件、PVP 插件、UI 插件。
- `src/states.rs` 定义全局 `AppState` 与 `RoomState`。

主要状态：

- `Loading`
- `MainMenu`
- `InGame`
- `MultiplayerMenu`
- `CoopMenu`
- `CoopLobby`
- `CoopGame`
- `PvpMenu`
- `PvpLobby`
- `PvpGame`
- `PvpResult`
- `Paused`
- `RewardSelect`
- `AugmentSelect`
- `LevelUpSelect`
- `Shop`
- `EventRoom`
- `GameOver`
- `Victory`

房间状态：

- `Idle`
- `Locked`
- `Cleared`
- `BossFight`

顶层插件职责：

- `core`：资源、输入、事件、音频、镜头、存档、成就、本地调试。
- `data`：加载 RON 配置，构造 `GameDataRegistry`。
- `gameplay`：地图、玩家、敌人、战斗、掉落、成长、奖励、事件房、商店、强化、特效。
- `coop`：Lightyear 合作闯关网络、运行时、UI。
- `pvp`：自写 UDP PVP 网络、运行时、UI。
- `ui`：主菜单、HUD、暂停、商店、奖励、强化、升级、事件房、胜败界面。

## 6. 单机模式当前功能与玩法

### 6.1 游戏流程

单机从主菜单的“单人游戏”进入。入口逻辑会插入 `FloorNumber(1)` 和 `EnemySpawnCount { current: 0 }`，然后切换到 `AppState::InGame`。

进入 `InGame` 后：

1. `MapPlugin` 生成或恢复楼层布局。
2. `PlayerPlugin` 生成玩家实体。
3. `EnemySystemsPlugin` 根据当前房间类型生成敌人或 Boss。
4. 玩家清理房间、拾取金币/XP、触发奖励或事件。
5. 房间清空后开门。
6. Boss 房清空后提供 Boss 强化奖励并生成传送门。
7. 玩家按 `E` 进入下一层，最终进入 `Victory`。
8. 玩家死亡则进入 `GameOver`。

当前单机循环已经覆盖完整 roguelike run 的主流程。

### 6.2 输入与控制

输入由 `src/core/input.rs` 统一收集成 `PlayerInputState`。

当前单机主要输入：

- WASD 或方向键：移动。
- 鼠标位置：世界坐标瞄准。
- 鼠标左键或 `J`：近战。
- 鼠标右键：远程。
- `Space`：冲刺。
- `E`：交互。
- `Esc`：暂停。
- `1 / 2 / 3 / 4`：技能槽。
- `B`：在商店房打开商店。
- `F5`：存档。
- `F9`：读档。

主菜单 UI 上也显示了核心操作提示。

### 6.3 地图与房间系统

房间类型定义在 `src/gameplay/map/room.rs`：

- `Start`
- `Normal`
- `Shop`
- `Reward`
- `Event`
- `Elite`
- `Boss`

当前 `assets/configs/rooms.ron` 中 `room_sequence` 为空，因此默认走程序化分支生成。

程序化生成规则大致如下：

- 楼层从 `Start` 房开始。
- 第一个战斗层固定为普通房。
- 中间层生成 2 或 3 条分支。
- 最后一层分支是 Boss 房。
- 中间层候选类型包括 Normal、Shop、Event、Reward、Elite。
- Elite 从第 2 层开始进入候选池。
- Reward 每层最多保留一个，多余 Reward 会被转成 Normal。

`RoomState` 用于控制房间是否锁门：

- 普通房、精英房、Boss 房进入后会锁定。
- 清空敌人后设置为 `Cleared`。
- Start、Reward、Shop、Event 主要通过交互和专用状态推进。

门系统会根据当前房间连接生成门和标签。单机房间切换只在 `InGame` 中执行，使用 `RoomTransition` 处理淡入淡出和玩家重定位。

### 6.4 玩家实体与基础属性

玩家由 `src/gameplay/player/systems.rs` 生成。

当前玩家初始组件包括：

- `Player`
- `LocalControlled`
- `TeamMarker(Team::Player)`
- `Health`
- `Energy`
- `Gold`
- `Combo`
- `SkillSlots`
- `PlayerSkillState`
- `PlayerDriveInput`
- `Velocity`
- `MoveSpeed`
- `AttackPower`
- `FacingDirection`
- `AnimationState`
- `CritChance`
- `RewardModifiers`
- `AugmentInventory`
- `PlayerLevel`
- 攻击/远程/冲刺/技能冷却
- 无敌计时器
- 冲刺状态
- 受击盒
- 闪烁效果
- 击退

关键初始配置来自 `assets/configs/player.ron`：

- 最大生命：100
- 移速：260
- 攻击力：18
- 近战冷却：0.70 秒
- 远程冷却：0.80 秒
- 冲刺冷却：1.2 秒
- 冲刺速度：680
- 冲刺持续：0.12 秒
- 无敌时间：0.35 秒
- 暴击率：5%
- 能量上限：100

### 6.5 近战、远程、冲刺与连击

近战：

- 左键或 `J` 触发。
- 生成近战斩击视觉和 hitbox。
- 受攻击力、暴击、近战成长、强化影响。
- 近战命中可给能量。
- 相关强化包括吸血斩、重击、连击加速、旋风斩、破甲、反弹、剑气波、处刑者等。

远程：

- 右键触发。
- 生成玩家投射物。
- 受远程成长和强化影响。
- 远程命中可给能量。
- 相关强化包括穿透弹、弹速提升、额外弹、追踪弹、连锁闪电、散射、弹幕风暴、冰冻弹等。

冲刺：

- `Space` 触发。
- 有短时位移和无敌窗口。
- 与完美冲刺回能、冲刺轨迹、冲刺护盾、闪现、击杀刷新冲刺等强化联动。

连击：

- `Combo` 有时间窗口。
- 连击可以触发额外能量或攻速类强化。

### 6.6 能量与主动技能

当前能量系统启用：`ENERGY_SYSTEM_ENABLED = true`。

能量来源包括：

- 近战命中。
- 远程命中。
- 击杀。
- 精英击杀。
- 完美冲刺。
- 连击奖励。

技能槽定义在 `SkillSlots` 中：

- 1 号槽：`SwordArc`，剑气斩，默认解锁。
- 2 号槽：`MarkedHunt`，标记猎杀，初始未解锁。
- 3 号槽：`LightningDash`，闪电冲刺，初始未解锁。
- 4 号槽：`Relic`，遗物主动，初始未解锁。

技能系统位于 `src/gameplay/skills/`。当前系统只在 `AppState::InGame` 运行。也就是说，合作模式里虽然玩家和战斗系统部分复用，但主动技能链路没有在 `CoopGame` 下运行。

### 6.7 敌人系统

敌人类型定义在 `src/gameplay/enemy/components.rs`：

- `MeleeChaser`
- `RangedShooter`
- `Charger`
- `Flanker`
- `Sniper`
- `SupportCaster`
- `Bomber`
- `Shielder`
- `Summoner`
- `Boss`

敌人池随楼层扩展：

- 第 1 层：近战追踪、远程射手、冲锋怪。
- 第 2 层：加入侧翼怪、自爆怪。
- 第 3 层：加入狙击手、盾卫。
- 第 4 层：加入辅助法师、召唤师。

配置来自 `assets/configs/enemies.ron`。当前小怪数值大致如下：

| 类型 | 生命 | 移速 | 伤害 | 特征 |
| --- | ---: | ---: | ---: | --- |
| MeleeChaser | 44 | 172 | 14 | 追踪近战 |
| RangedShooter | 34 | 130 | 11 | 保持距离并发射弹幕 |
| Charger | 62 | 120 | 21 | 蓄力冲锋 |
| Flanker | 46 | 195 | 15 | 侧翼接近和突进 |
| Sniper | 44 | 108 | 17 | 长距离瞄准射击 |
| SupportCaster | 38 | 108 | 9 | 给友军 buff，并发射慢弹 |
| Bomber | 30 | 185 | 28 | 贴近后自爆 |
| Shielder | 72 | 80 | 12 | 正面盾牌抵挡远程 |
| Summoner | 28 | 95 | 8 | 召唤小怪 |

普通房按楼层和配置生成敌人数量。精英房生成更强敌人。击杀敌人会触发死亡事件、掉落、能量、回血、房间清空检测等。

### 6.8 精英词缀

精英系统当前有 6 种词缀：

- `Swift`：迅捷，提升速度并降低攻击间隔。
- `Splitting`：分裂，死亡后生成两个较弱同类怪，不掉落。
- `Shielded`：护盾，吸收一次伤害。
- `Vampiric`：吸血，命中玩家时回复生命。
- `Berserk`：狂暴，低血量时伤害提高并变红。
- `Teleporting`：闪现，周期性瞬移。

精英有视觉标记：

- 体型放大。
- 发光。
- 中文词缀标签。
- 小怪血条。

### 6.9 Boss 系统

Boss 原型由楼层决定：

- 第 1 层：`Floor1Guardian`
- 第 2 层：`MirrorWarden`
- 第 3 层：`TideHunter`
- 第 4 层及以后：`CubeCore`

Boss 数值来自 `assets/configs/boss.ron`：

| 楼层 | 生命 | 移速 | 接触伤害 | 弹速 |
| --- | ---: | ---: | ---: | ---: |
| 1 | 330 | 95 | 14 | 430 |
| 2 | 450 | 130 | 15 | 470 |
| 3 | 600 | 175 | 18 | 505 |
| 4 | 620 | 82 | 16 | 540 |

Boss 共有阶段系统，血量低于配置阈值会进入新阶段并发送 `BossPhaseChangeEvent`。

具体行为：

- `Floor1Guardian`：有方向性防御，正面受击减伤，并显示盾牌指示器。
- `MirrorWarden`：在锚点间瞬移，生成分身，分身会发射弹幕但不可被伤害。
- `TideHunter`：有 Stalk、Telegraph、ShadowDash、Reposition、Stunned 等阶段，会暗影冲刺并留下伤害轨迹，存在冲刺反制窗口。
- `CubeCore`：有环绕子核心，子核心存在时主体免疫；阶段推进时可重生核心，并有环形、十字、螺旋、墙型等弹幕模式。

Boss 死亡后：

- 清房。
- 触发 Boss 奖励。
- 根据规则治疗玩家一部分生命。
- 生成 Boss 传送门。
- 玩家靠近传送门按 `E` 进入下一层或通关。

### 6.10 掉落、金币与 XP

掉落系统位于 `src/gameplay/drops/mod.rs`。

敌人死亡后生成：

- 金币掉落。
- XP 掉落。

掉落具有：

- 物理散射。
- 磁吸。
- 拾取范围。
- 生命周期。
- 拾取文本。

基础掉落规则：

- 普通怪基础金币随楼层增加。
- 精英有额外金币奖励，当前 `elite_gold_bonus` 为 18。
- Boss 掉落更多金币和 XP。
- 第 3 层以后 Boss/精英掉落数量翻倍。
- `GoldBonus` 和 `XpBonus` 强化会提升收益。

XP 用于 `PlayerLevel`。默认等级为 1，初始升级需求为 30，之后递增。

### 6.11 升级系统

升级系统位于 `src/gameplay/progression/experience.rs` 与 `src/ui/levelup_select.rs`。

升级时生成 4 个选项：

- 第 1 个固定是回血。
- 另外 3 个从属性池随机抽取。

属性池包括：

- 攻击力 +3。
- 生命上限 +15。
- 移动速度 +15。
- 暴击率 +5%。
- 攻速提升。
- 冲刺冷却降低。

升级 UI 支持数字键和点击选择。选择后返回原状态，通常是 `InGame`。

### 6.12 强化系统

当前强化系统位于 `src/gameplay/augment/`，旧 `RuneLoadout` 已由 `AugmentInventory` 替代。

强化配置在 `assets/configs/augments.ron`，共 30 个：

近战类 8 个：

- 吸血斩
- 重击
- 连击加速
- 旋风斩
- 破甲
- 反弹
- 剑气波
- 处刑者

远程类 8 个：

- 穿透弹
- 弹速提升
- 额外弹
- 追踪弹
- 连锁闪电
- 散射
- 弹幕风暴
- 冰冻弹

机动类 6 个：

- 冲刺留痕
- 冲刺回能
- 无敌延长
- 冲刺重置
- 冲刺护盾
- 闪现

通用类 8 个：

- 金币加成
- 经验加成
- 拾取范围
- 荆棘
- 击杀回血
- 暴击强化
- 不死鸟
- 贪婪

每个强化最多 2 层：

- 第 1 次获得：普通效果。
- 第 2 次获得：升级效果。
- 第 3 次及以后保持 2 层。

强化来源：

- 普通房清空后概率出现强化选择。
- 精英房清空后必定出现强化选择。
- Boss 清空后出现更强的 Elite/Legendary 强化池。
- 奖励房圣所。
- 商店购买。
- 事件房奖励。

### 6.13 奖励房与圣所

当前奖励房已不再是旧 rune/curse/祝福体系，而是“圣所”。

进入 Reward 房后会打开 `RewardSelect`，提供三类服务：

1. 完全恢复：恢复生命和能量。
2. 强化服务：
   - 如果玩家已有单层强化，则列出最多 3 个可升级项。
   - 如果没有可升级项，则从 Elite/Legendary 中提供觉醒选择。
3. 启示：
   - 直接提升玩家等级。
   - 进入升级选择 UI。

完成圣所选择后，房间设为 `Cleared`。

### 6.14 事件房

当前事件房类型定义在 `src/gameplay/event_room/mod.rs`，共有 10 类：

- 压力板试炼。
- 机关顺序。
- 陷阱求生。
- 赌徒。
- 血契。
- 宝箱。
- 治愈泉。
- 流动商贩。
- 限时清剿。
- 精英决斗。

谜题类：

- 压力板：站在压力板上累计时间。
- 机关顺序：靠近机关按 E，按 1-2-3 顺序触发。
- 陷阱求生：躲避周期性陷阱并撑过时间。

非战斗事件：

- 赌徒：花 50 金币随机获得强化。
- 血契：失去 30% 当前生命，获得强化。
- 宝箱：免费强化并获得 30 金币。
- 治愈泉：恢复 40% 最大生命。
- 流动商贩：半价购买强化。

战斗事件：

- 限时清剿：30 秒内清空房间可获得精英强化。
- 精英决斗：击败单个精英，获得精英强化。

旧 `CurseAltar` 已从当前事件房代码中移除。

### 6.15 商店系统

商店位于 `src/gameplay/shop/mod.rs`，只在 `InGame` 中运行。

进入 Shop 房：

- 生成商店 kiosk。
- 首次进入会自动打开商店。
- 可用 `E` 或 `B` 打开。

商店分为：

- 属性商品。
- 强化商品。
- 工具商品。

属性商品包括：

- 治疗。
- 最大生命。
- 攻击力。
- 冲刺冷却。
- 移动速度。
- 能量上限。
- 暴击率。
- 攻速。

强化商品直接使用 `AugmentId`。工具商品当前主要是治疗药水。刷新第一次免费，后续费用递增。

### 6.16 UI 状态

UI 插件当前覆盖：

- 主菜单。
- 联机菜单入口。
- HUD。
- 暂停菜单。
- 商店。
- 奖励选择。
- 强化选择。
- 升级选择。
- 事件房。
- 游戏失败。
- 胜利。
- 自定义光标和准星。
- 成就通知。

HUD 显示：

- 生命条和生命文字。
- XP 条和等级。
- 金币。
- 能量。
- 冲刺冷却。
- 技能槽。
- 楼层。
- 房间类型和状态。
- 敌人数量。
- 当前提示。
- Boss 血条。
- 楼层进度。

HUD 在 `InGame`、`RewardSelect`、`CoopGame` 中都会挂载，但具体数据源在单机和 Coop 下不同。

### 6.17 资源与美术状态

核心资源由 `src/core/assets.rs` 加载。

当前加载：

- 字体：`fonts/main_font.ttf`
- 玩家贴图：`textures/player_hero.png`
- 近战斩击图集：`textures/effects/melee_slash_sprites.png`
- 程序生成光标。
- 程序生成准星。
- 1x1 白色贴图。
- 敌人贴图映射：
  - `MeleeChaser -> textures/enemies/melee_chaser.png`
  - `RangedShooter -> textures/enemies/ranged_shooter.png`
- Boss 贴图映射：
  - `Floor1Guardian -> textures/bosses/floor1_guardian.png`
- 房间背景：
  - `textures/backgrounds/room_bg_default.jpg`

注意：

- `assets/configs/game_balance.ron` 当前 `use_sprite_textures: false`，因此小怪和 Boss 的新贴图虽然被加载，但默认不会用于敌人/Boss 渲染。
- 房间背景会被 `TilesPlugin` 直接用于房间地板。
- 新增背景、Boss、敌人贴图目录当前是未跟踪文件，其他机器或干净 clone 不一定具备这些资源。
- optional 贴图不参与 Loading 阶段 readiness 检查，字体、玩家、斩击图集加载完成后就进入主菜单。

### 6.18 音频状态

音频模块存在较完整框架：

- 程序化生成 SFX。
- 有 `SfxEvent` 与 `SfxKind`。
- 有 BGM 同步接口。
- 有音量配置结构。

但当前 `assets/configs/audio.ron` 中音量基本为 0，且 `cargo check` 显示 `SfxHandles` 和 BGM 相关代码未被实际使用。当前实际体验可以视为“音频框架存在，但音频基本关闭/未接通”。

### 6.19 存档与成就

存档系统位于 `src/core/save.rs`。

热键：

- `F5` 存档。
- `F9` 读档。

保存内容：

- 版本号。
- 楼层。
- 玩家生命、能量、金币、移速、攻击、暴击。
- `RewardModifiers`。
- 攻击/冲刺/远程冷却。
- 敌人生成计数。
- 已解锁成就。

未保存内容：

- 当前完整楼层布局。
- 当前房间 ID。
- 当前房间状态。
- `AugmentInventory`。
- `PlayerLevel`。
- XP。
- 事件房状态。
- 商店缓存状态。
- 掉落物。
- Coop/PVP 状态。

成就系统包括：

- FirstBlood
- EliteSlayer
- Combo10
- Rich
- Shopper
- PuzzleSolver
- BossSlayer
- Untouchable
- Victory

## 7. 多人模式设计架构

### 7.1 联机入口

主菜单点击“联机游戏”进入 `MultiplayerMenu`。

联机菜单有两个方向：

- 玩家合作，一起闯关：进入 `CoopMenu`。
- 玩家对抗，2P PVP：进入 `PvpMenu`。

这两个模式不是同一套网络架构。

## 8. 合作模式架构

### 8.1 网络栈

合作模式使用 Lightyear `0.17.1`。

关键参数：

- UDP 端口：`3457`
- 固定协议 ID：`COOP_PROTOCOL_ID`
- 固定私钥：`COOP_PRIVATE_KEY`
- Host ClientId：1
- Remote ClientId：2

关键文件：

- `src/coop/net.rs`
- `src/coop/runtime.rs`
- `src/coop/components.rs`
- `src/coop/ui.rs`

网络模式：

- `NetMode::None`
- `NetMode::Host`
- `NetMode::Client`

Host 会启动 server 和本地 client。Client 连接 Host。Host 是游戏权威。

### 8.2 协议与同步对象

合作模式注册了：

- `CoopCommandChannel`：可靠有序命令通道。
- Client -> Server 命令：
  - 选择奖励。
  - 选择猜拳。
  - 购买商店商品。
  - 刷新商店。
- Server -> Client 消息：
  - `CoopDamageEvent`，用于客户端显示伤害数字。

同步组件包括：

- `Player`
- `Health`
- `Gold`
- `MoveSpeed`
- `FacingDirection`
- `AnimationState`
- `CoopParticipant`
- `PlayerSlot`
- `GhostState`
- `CoopNetPosition`
- `CoopNetVelocity`
- `CoopNetRotation`
- `CoopMeleeFlashState`
- `CoopDashVisualState`
- `CoopSessionState`
- `Enemy`
- `EnemyKind`
- `Projectile`
- `Door`

Host 端对本地/远端玩家分别建立 controlled replication。

### 8.3 输入模型

客户端输入结构为 `CoopInputState`，包含：

- 移动轴。
- 瞄准世界坐标。
- 近战按下/持续。
- 远程按下/持续。
- 冲刺。
- 交互。
- 暂停。
- 商店。
- 菜单确认/取消。

代码中有专门逻辑合并边缘事件，避免多个 tick 到达时后一个 false 覆盖前一个 true。这说明历史上曾修复过 P2 冲刺/E 键丢失一类问题。

### 8.4 Host 权威运行时

`CoopRuntimePlugin` 只在 Host 权威下执行核心模拟。

Host 启动对局时：

1. 等待本地和远端都连接。
2. 插入 `FloorNumber(1)`。
3. 插入 `VisitedRooms`、`RewardRoomGoldBonusSeen`、`RoomState`。
4. 生成楼层布局。
5. 将 `Event` 房归一化为 `Normal` 房。
6. 生成 P1 和 P2 玩家。
7. 生成 `CoopSessionState` 并复制给客户端。

Coop 运行时负责：

- 暂停切换。
- 输入缓冲。
- 玩家死亡转 Ghost。
- 全灭/胜利判断。
- 房间阶段进入。
- 清房奖励阶段。
- 奖励命令处理。
- 商店退出。
- 门交互和路线选择。
- 猜拳解决路线争议。
- 给可复制实体打 replication 标记。
- 同步网络位置、速度、旋转和 dash 冷却。
- 广播伤害事件。
- 清理断线会话。

### 8.5 Coop 阶段机

`CoopPhase` 定义：

- `None`
- `Paused`
- `Reward`
- `DoorChoice`
- `Rps`
- `Shop`
- `MatchOver`

这些阶段用于控制玩家移动、UI 模态框和 Host 侧逻辑。

玩家移动会被以下阶段阻止：

- `Paused`
- `Reward`
- `Rps`
- `Shop`
- `MatchOver`

`DoorChoice` 不阻止移动，因为玩家需要走到门边选择路线。

### 8.6 Coop 奖励与复活

Coop 使用自己的奖励状态：

- `CoopRewardMode::SingleBuff`
- `CoopRewardMode::HealOrBuff`
- `CoopRewardMode::DualBuff`
- `CoopRewardMode::LoneSurvivor`

选项类型：

- `Buff(RewardType)`
- `Rest`
- `Revive`

如果一名玩家死亡，玩家进入 `GhostState::Ghost`。孤存者可能获得休息、复活或强化选项。复活会把玩家从 Ghost 恢复为 Alive，并恢复一定生命。

这里需要注意：Coop 奖励仍主要基于 `RewardType` 和 `RewardModifiers`，不是当前单机的 `AugmentInventory` 主链路。

### 8.7 Coop 商店

Coop 有自己的商店状态：

- 每个玩家有独立 `PlayerShopState`。
- 商品类型为 `CoopShopItem`。
- 支持购买和刷新。
- 商品包括治疗、生命上限、攻击、冲刺冷却、移速、能量上限、暴击、攻速。

Coop 商店与单机 `ShopPlugin` 不是同一套 UI/逻辑，虽然部分数值函数和奖励应用逻辑有共享。

### 8.8 Coop 路线选择与猜拳

Coop 门交互逻辑支持：

- 玩家靠近门按 `E` 锁定路线。
- 如果双方选择不同门，进入 `CoopPhase::Rps`。
- 猜拳选择包括石头、布、剪刀。
- 超时会自动补全选择。
- 胜者路线决定最终前进方向。

### 8.9 Coop 客户端 UI 与视觉预测

客户端 UI 负责：

- 合作菜单。
- 大厅。
- 游戏内状态面板。
- modal 式奖励、商店、猜拳界面。
- replicated 玩家视觉附着。
- replicated 门标签。
- 远端血条。
- 本地动画预测。
- 伤害数字显示。

代码中包含 duplicate filtering，避免 Host 本地实体和 replicated 实体在客户端重复显示。

### 8.10 Coop 与单机系统复用边界

已经复用的系统：

- 玩家移动/攻击/远程/冲刺的一部分。
- 敌人 AI。
- 敌人生成。
- 战斗 hitbox/damage/projectile。
- 掉落。
- 强化效果的一部分。
- 特效。
- 门视觉。
- HUD 的一部分。

没有完全复用或存在分叉的系统：

- 单机升级系统只在 `InGame` 跑。
- 单机技能系统只在 `InGame` 跑。
- 单机奖励房圣所不是 Coop 主奖励流。
- 单机商店与 Coop 商店是两套状态。
- 单机事件房在 Coop 布局中被转换成普通房。
- 单机存档不覆盖 Coop。

## 9. PVP 模式架构

### 9.1 网络栈

PVP 使用自写 UDP 网络，端口为 `3456`。

关键文件：

- `src/pvp/net.rs`
- `src/pvp/systems.rs`
- `src/pvp/components.rs`
- `src/pvp/ui.rs`

PVP 网络模式：

- `NetMode::None`
- `NetMode::Host`
- `NetMode::Client`

协议消息：

- `Hello`
- `Welcome { your_id }`
- `Input(PvpInputMsg)`
- `State(PvpStateMsg)`
- `Fire(PvpFireMsg)`
- `Result { winner }`

### 9.2 PVP 规则

UI 文案明确说明：

- 局域网 2P 对战。
- 每人 3 条命。
- 无技能。
- 只保留移动、近战、远程。

Host 负责模拟：

- 两个玩家移动。
- 近战命中。
- 远程开火。
- 死亡/重生。
- 胜负。
- 30Hz 状态广播。

Client 负责：

- 发送本地输入。
- 应用 Host 状态。
- 本地预测。
- 插值显示。
- 子弹视觉。

PVP 与单机/Coop 的 roguelike 内容基本没有共享成长和房间流程。

## 10. 配置状态

当前 `assets/configs/` 主要配置：

- `audio.ron`
- `augments.ron`
- `boss.ron`
- `effects.ron`
- `enemies.ron`
- `game_balance.ron`
- `player.ron`
- `rewards.ron`
- `rooms.ron`

已删除：

- `runes.ron`
- `curses.ron`

关键 `game_balance.ron` 状态：

- 总楼层：4。
- 每层房间数：7。
- 每层难度增长：0.16。
- 普通房基础敌人数：4。
- 精英概率：0.18。
- 精英生命倍率：2.0。
- 精英伤害倍率：1.55。
- 精英金币奖励：18。
- `use_sprite_textures: false`。

配置加载逻辑：

- `player`、`enemies`、`bosses`、`rewards`、`rooms`、`game_balance` 是必需配置。
- `augments`、`audio`、`effects` 是可选或有默认。
- 任一必需配置加载失败时，当前实现会整体回退 `default_registry()`。

这会带来一个重要问题：如果某个必需 RON 文件格式错误，游戏不是只回退该配置，而是整个 registry 全部回退默认值。默认值与当前实际 RON 已经存在差异。

## 11. 文档状态

文档整体覆盖面较广，但当前同步程度不一致。

相对可信的文档：

- `docs/02_architecture.md`
- `docs/03_module_design.md`
- `docs/04_api_and_data_model.md`
- `docs/06_multiplayer_and_risks.md`
- `docs/architecture_refactor_suggestions.md`

明显滞后的文档：

- `CLAUDE.md` 仍描述 `src/gameplay/rune/`、`src/gameplay/curse/`、`runes.ron`、`curses.ron` 为当前系统。
- `docs/01_build_and_run.md` 仍提到旧的 `ReceivedCharacter` 和旧测试数量。
- 部分 docs/README 中的 Rust 文件数量不是当前 107。
- `docs/meeting_briefing.md` 仍提到“5 curses”之类旧内容。

历史性文档中出现 rune/curse 是合理的，但面向当前状态的文档仍保留这些描述，会误导后续维护者。

## 12. 潜在问题与风险

### 12.1 高风险：文档与代码严重不同步

当前工作树已经删除 rune/curse 源码和配置，但 `CLAUDE.md` 仍把它们描述为当前架构。

影响：

- 新维护者会按不存在的模块寻找代码。
- 后续计划可能基于错误架构做判断。
- 与 `PLANS.md` 和当前未提交重构之间也存在语义错位。

建议后续优先更新面向当前状态的文档，并明确历史文档与当前实现的边界。

### 12.2 高风险：Coop 与单机成长系统分叉

单机当前主成长链路是：

- XP。
- LevelUp。
- AugmentInventory。
- 圣所。
- AugmentSelect。

Coop 当前主链路仍是：

- `RewardType`。
- `RewardModifiers`。
- `CoopRewardMode`。
- Coop 专用商店。

影响：

- 单机新增强化不会自然进入 Coop。
- Coop 平衡和单机平衡会越来越分离。
- 文档中如果说 Coop 复用单机玩法，需要加限定。

具体代码现象：

- `ProgressionPlugin` 只在 `InGame` 运行。
- `SkillsPlugin` 只在 `InGame` 运行。
- Coop 会生成 drops，但 XP 消费和升级 UI 并不是 Coop 主流程。
- Event 房在 Coop 布局里被归一化为 Normal。

### 12.3 高风险：存档落后于当前玩法

存档不保存 `AugmentInventory` 和 `PlayerLevel`。这两个现在是单机成长核心。

影响：

- F5/F9 后可能丢失强化和等级。
- 现代 run 的真实状态无法完整恢复。
- 存档对调试旧奖励修正有效，但对当前强化系统不足。

### 12.4 中风险：配置整体回退默认值

`try_load_all()` 对必需配置使用 `?`，任一必需配置失败就进入 `default_registry()`。

影响：

- 一个配置错误会导致所有配置回退。
- 默认值与当前 RON 不完全一致。
- 运行时可能“看似正常”，但数值变成旧值。

### 12.5 中风险：资源接入状态不可复现

当前新增贴图目录是未跟踪状态。`cargo check/test` 不依赖资源文件，所以不能证明运行时资源在干净环境中存在。

影响：

- 别的机器上运行可能缺少背景、敌人或 Boss 贴图。
- `use_sprite_textures` 目前为 false，敌人/Boss 贴图默认不启用，容易误以为资产接入无效。
- optional 贴图不参与 Loading readiness 检查，缺失时更偏运行时表现问题。

### 12.6 中风险：音频代码存在但未接通

当前音频配置音量为 0，且编译有音频句柄 dead code 警告。

影响：

- 文档若宣称已有完整音效体验，会不准确。
- SFX 事件和句柄之间的路径需要重新确认。

### 12.7 中风险：大型文件职责过重

明显复杂热点：

- `src/coop/runtime.rs`
- `src/coop/ui.rs`
- `src/gameplay/enemy/systems.rs`
- `src/gameplay/enemy/boss.rs`
- `src/ui/hud.rs`
- `src/gameplay/rewards/systems.rs`

影响：

- 回归风险高。
- 状态机、UI、网络同步、玩法规则耦合。
- 测试局部行为时上下文成本高。

### 12.8 中风险：大量 dead code 允许项会降低警告价值

源码里仍有大量 `#![allow(dead_code)]` 和 `#[allow(dead_code)]`。

影响：

- 无用代码和过时字段不容易暴露。
- 当前 `cargo check` 已经重新出现 dead code 警告，说明警告治理状态退化。

### 12.9 中风险：若干 `unwrap()` 依赖隐含资源一致性

代码中仍有一些 `layout.room(current_room.0).unwrap()` 或基于前置假设的 `unwrap()`。

多数在正常流程中可能安全，但如果资源状态被存档/联机/切状态打乱，仍可能 panic。

### 12.10 中风险：随机数来源混用

项目有 `GameRng` 资源，但一些地方会临时 `GameRng::default()`。当前 `GameRng::default()` 来自系统熵，不是固定种子。

影响：

- 如果未来需要可复现 run、回放、同步调试或固定种子测试，局部熵源会破坏可重复性。
- 对普通游玩不是立即 bug，但对网络和调试是架构风险。

### 12.11 低到中风险：当前 Rust edition 与项目说明可能不一致

`Cargo.toml` 是 Rust edition 2024，而项目执行说明中仍提到 Rust 2021 风格。实际代码可以编译，但文档和执行约定不完全一致。

## 13. 当前仓库可用性判断

单机模式：

- 可编译。
- 测试通过。
- 玩法闭环完整。
- 当前功能量已经足以作为主要可玩版本。
- 主要问题是存档、文档同步、音频、资产启用状态。

合作模式：

- 架构清晰，Host 权威设计合理。
- 已有输入同步、实体复制、阶段机、奖励、商店、复活、路线选择、猜拳。
- 与最新单机成长系统没有完全统一。
- 复杂度高，后续改动需要重点回归测试。

PVP 模式：

- 独立、轻量、规则清楚。
- 适合作为局域网原型。
- 与主 roguelike 玩法解耦明显。

文档：

- 历史信息丰富。
- 当前状态文档需要一次系统性同步，尤其是 rune/curse 删除、强化系统、Coop 分叉、资源状态、测试数量、警告状态。

## 14. 验证记录

本次报告前执行了：

```text
git status --short
git diff --stat
git log --oneline --decorate --max-count=25
cargo check --quiet
cargo test --quiet
```

结果：

- `git status --short`：工作树有大量未提交修改、删除和未跟踪文件。
- `git diff --stat`：39 个文件变化，1236 行新增，2531 行删除。
- `cargo check --quiet`：成功，但有音频 dead code 警告。
- `cargo test --quiet`：成功，45 passed，0 failed；同样有音频 dead code 警告。

## 15. 最终评价

当前仓库的单机模式已经从“原型”进入“内容较完整的可玩版本”阶段。核心玩法、成长、房间、敌人、Boss、奖励、事件、商店和 UI 都已经成形。最近的未提交重构使系统从旧 rune/curse 方案转向更直接的 `AugmentInventory` 强化体系，这个方向在代码上已经基本落地。

多人合作模式是一个功能较多的主机权威网络原型，架构方向正确，但它没有完全跟上单机最新成长体系。PVP 则是另一套独立对战原型，范围较小、规则明确。

目前最大的工程风险不是“代码无法运行”，而是“仓库状态和文档叙述不一致”。如果继续开发，建议先把当前未提交重构的意图、边界和验证结果固化，再同步文档，之后再处理 Coop 与单机成长系统的统一问题。
