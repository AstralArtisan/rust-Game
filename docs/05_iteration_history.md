# 迭代经历与设计演化

- 适用版本：当前工作树（HEAD `0553e76b`）
- 最后校验：2026-04-06；`cargo check` 通过，`cargo test` 33 项通过
- 关联源码：`rust_game_codex_requirements.txt`、`git log`、`src/`、`docs/project_overview_and_coop_review.md`
- 实验性内容：包含。联机相关阶段记录的是“原型整合”而非稳定版本发布

## 1. 总览
这个仓库的演化不是线性“做完单机再做联机”，而是围绕课程项目目标，在很短的时间内交错推进：

- 单机主循环骨架
- 联机试探
- 战斗/成长深化
- 表现替换
- 多人合并
- 版本封存

下表按真实提交整理主线阶段。

| 阶段 | 时间 | 关键提交 | 主题 |
| --- | --- | --- | --- |
| 阶段 0 | 需求基线 | `rust_game_codex_requirements.txt` | 明确课程项目的功能边界与架构要求 |
| 阶段 1 | 2026-03-18 | `8a8758d7` | 初始工程与单机骨架 |
| 阶段 2 | 2026-03-18 | `bcdfc23e`、`60619dbe` | 联机测试试探 |
| 阶段 3 | 2026-03-18 | `29a01580` | 战斗平衡与多楼层推进 |
| 阶段 4 | 2026-03-18 | `541538b2`、`3ab30e32` | 角色表现升级与敌人成长细化 |
| 阶段 5 | 2026-03-18 | `bd3add48` | 多人能力与 Roguelike 主线合并 |
| 阶段 6 | 2026-03-28 ~ 2026-03-30 | `91a8bb1f`、`aa90cf3c` | 工作版本保存与文档审视期 |

## 2. 阶段 0：需求基线
时间：形成于项目开发前期  
基线材料：`rust_game_codex_requirements.txt`

### 目标
- 用 Rust 完成一个可演示的 2D 俯视角动作 Roguelike
- 必须体现工程结构、模块划分、状态管理、配置驱动和可扩展性
- 最终不仅要“能玩”，还要“能讲清楚设计”

### 对后续结构的直接影响
- 促成了 `src/app.rs` + 多插件装配结构
- 明确了 `core / data / gameplay / ui` 的模块边界
- 规定了状态机、房间推进、奖励系统、Boss、多类敌人、HUD 等最低交付项
- 为后续把单机和联机都塞进同一工程提供了“必须模块化”的约束

### 仍然可见的遗产
- 当前目录结构仍明显沿着需求文档建议的骨架展开
- `states.rs`、`data/definitions.rs`、`gameplay/*` 的职责划分与需求文档高度一致

## 3. 阶段 1：初始工程与单机骨架
提交：`8a8758d7`  
日期：2026-03-18  
标题：`first commit`

### 目标
- 建立基础工程
- 形成最小可运行的游戏框架

### 典型成果
- 初步建立 Bevy 项目
- 引入基础目录结构
- 确立“从主菜单进入游戏”的主线路

### 设计意义
这是“所有后续模块都围绕一个统一 App 生长”的起点。虽然提交标题非常朴素，但它决定了后续不是做多个独立 demo，而是做一个能持续叠加模块的工程。

### 遗留问题
- 此时仍接近骨架阶段，玩法、敌人、成长、联机都还不完整

## 4. 阶段 2：联机测试试探
提交：`bcdfc23e`、`60619dbe`  
日期：2026-03-18  
标题：`second commit 联机测试`、`third commit 联机测试`

### 目标
- 尝试多人通信能力
- 为后续合作联机和对战联机打基础

### 典型成果
- 开始引入“多人模式”这个独立问题域
- 把联机作为正式开发方向，而不是课设收尾时才临时附加

### 设计意义
这一步让仓库从“单机动作小游戏”变成“单机 + 多人扩展”的工程。后续 `Coop` 与 `PVP` 的存在，根子都在这一阶段的试探。

### 遗留问题
- 早期联机试验通常会带来协议、生命周期和 UI 路径的多次重写
- 当前仓库里保留的部分技术债，本质上都与这类快速试探有关

## 5. 阶段 3：战斗平衡与多楼层推进
提交：`29a01580`  
日期：2026-03-18  
标题：`Tune combat balance and add multi-floor progression`

### 目标
- 把游戏从“单房间对打”推进为“多楼层 Roguelike 闭环”
- 调整战斗体验和成长节奏

### 典型成果
- 多楼层推进成为正式能力，而不是一次性关卡
- 平衡参数和成长曲线开始系统化
- `progression`、奖励、商店、楼层难度这些概念开始真正串起来

### 设计意义
这是项目从“动作原型”转向“Roguelike 主循环”的关键一步。后续的 Boss、奖励、商店、成就、存档都建立在多楼层推进语义之上。

### 遗留问题
- 多楼层推进意味着更多状态和更多横切数据，复杂度显著增加
- 这也推动了后续需要更强的配置驱动和共享规则层

## 6. 阶段 4：角色表现升级与敌人成长细化
提交：`541538b2`、`3ab30e32`  
日期：2026-03-18  
标题：`Replace player block with hero sprite`、`Refine combat progression and enemy behaviors`

### 目标
- 把玩家从占位块升级到更可展示的角色表现
- 让战斗、敌人行为、成长反馈更接近完整课程作品

### 典型成果
- 玩家视觉表现不再只是占位色块
- 敌人 AI、战斗节奏、成长反馈继续细化
- 项目从“可运行”进一步走向“可演示”

### 设计意义
这一步决定了项目不是单纯追求结构，而是开始兼顾答辩观感。`player/animation.rs`、敌人行为、特效、HUD 等系统的重要性由此上升。

### 遗留问题
- 表现层升级后，UI 与玩法状态耦合开始增加
- 这也解释了为什么 `ui/hud.rs` 会逐渐成为一个复杂度热点

## 7. 阶段 5：多人能力与 Roguelike 主线合并
提交：`bd3add48`  
日期：2026-03-18  
标题：`Merge upstream multiplayer features with roguelike gameplay`

### 目标
- 把上游多人能力与本地单机/成长/地图主线整合到一个统一仓库

### 典型成果
- `coop/` 与 `pvp/` 正式并入主工程
- `AppState` 扩展出 `MultiplayerMenu`、`CoopMenu`、`CoopLobby`、`CoopGame`、`PvpMenu`、`PvpLobby`、`PvpGame`、`PvpResult`
- 单机、Coop、PVP 三条路线共处同一应用

### 设计意义
这是当前仓库形态真正形成的阶段。此后仓库不再只是“单机主线 + 附加试验”，而是一个必须同时解释三种流程的综合工程。

### 遗留问题
- 双网络栈并存：Coop 用 Lightyear，PVP 用手写 UDP
- 玩法共享边界变复杂：哪些系统能在 Coop 里复用，哪些只能留在单机
- 文档和调试方式开始容易过时

## 8. 阶段 6：工作版本保存与审视期
提交：`91a8bb1f`、`aa90cf3c`  
日期：2026-03-28、2026-03-30  
标题：`Save current version`、`chore: save current working version`

### 目标
- 冻结当前工作版本
- 为审查、回归、文档化和后续交接提供稳定基线

### 典型成果
- 形成可回溯的保存点
- 允许围绕当前工作树做系统性审计与文档整理

### 设计意义
这一步说明项目已经进入“从写功能转向整理与交付”的阶段。当前这套工程交接文档，本质上就是建立在这个阶段的稳定保存点之上。

### 当前质量状态
- 相比旧文档阶段，测试已增长到 24 个单元测试
- 但编译告警仍然较多，说明项目进入了“功能可用、结构需继续收敛”的典型中后期状态

## 9. 演化结论
从提交历史看，项目的主要演进方向很清晰：

1. 先确立课程型模块化骨架。
2. 再尽早试探联机能力，避免最后临时拼接。
3. 在单机主循环形成后，补足多楼层、成长和表现层。
4. 最后把多人分支合并进来，形成当前的综合工程。

这解释了当前仓库的两个核心特征：

- 单机主循环骨架已经相对扎实
- 联机系统真实存在，但仍带有快速整合期的复杂度和技术债

---

## 10. Coop 联机调试第三轮（2026-04-02）

### 改动内容
- 输入过期清零：`CoopNetState` 新增 `latest_input_ticks` 和 `host_frame_counter`，`host_buffer_player_inputs` 检测 P2 输入超过 3 帧未更新时自动清零 `move_axis` 和 `held` 字段
- Despawn 重复 Replicated Player：`filter_replicated_player_duplicates` 从隐藏改为 despawn 非最佳实体
- EventReader drain：`capture_server_inputs` 和 `receive_coop_command_messages` 在 early return 时调用 `.clear()`
- 对齐 tick/replication rate：`server_replication_send_interval` 从 1/64s 改为 1/60s

### 目的与动机
用户反馈 Client 端松键后角色持续滑行（输入粘滞），且游玩数分钟后完全卡死。三个并行调查 Agent 一致定位到两个根因：(1) `latest_inputs` 中 P2 的 `move_axis` 在无新包到达时永不归零；(2) 重复的 Replicated Player 实体只被隐藏不被销毁，随网络波动无限累积。

### 关键决策
- 输入过期采用帧计数器而非时间戳，避免引入 `Time` 资源依赖，且与 FixedUpdate 的离散特性更匹配
- 重复实体改为 despawn 而非隐藏，因为隐藏实体仍参与所有 ECS 查询，是性能退化的直接原因
- `CoopNetVelocity` 复制暂时保留，因为同时被 Player 和 Projectile 使用，移除涉及面较广

### 已知问题 / 后续工作
- 需要实际双开测试验证粘滞和卡死是否彻底解决
- `cargo test` 32 项全部通过

如果要继续演进，最合理的下一步不是再无节制加新功能，而是围绕文档、规则抽象、联机生命周期和复杂度热点做收敛。

## 11. 战斗蓄力与技能槽位系统（2026-04-03）

### 改动内容
- 能量系统重做：从"自然回复 + 消耗回血/限制远程"改为"战斗蓄力"——通过近战命中、远程命中、击杀、完美冲刺、连击维持充能，蓄满后主动释放终结技
- 技能槽位系统：数字键 1-4 对应 HUD 底部技能栏，1-3 号位放终结技，4 号位预留
- 三种终结技实现：剑气斩（1号位，大范围近战）、标记猎杀（2号位，锁定追踪弹）、闪电冲刺（3号位，穿透冲刺）
- HUD 技能栏 UI：技能图标、按键提示、能量满时发光闪烁、锁定模式遮罩
- 输入绑定：`PlayerInputState` 新增 `skill_1_pressed` ~ `skill_4_pressed`，绑定数字键 1-4
- 配置更新：`player.ron` 新增充能参数（`charge_on_melee_hit`、`charge_on_ranged_hit` 等）
- 移除能量回血：`heal_energy_cost_per_s`、`heal_hp_per_s` 不再生效
- Bug 修复：`update_skill_bar_ui` 中 Query 冲突（`&mut Text` 和 `&mut BackgroundColor` 的多查询冲突），通过 `Without<>` 过滤器解决

### 目的与动机
原能量系统被禁用（`ENERGY_SYSTEM_ENABLED = false`），因为能量回血鼓励保守玩法、能量限制远程攻速体验不佳。重做为战斗蓄力系统，鼓励进攻、奖励操作技巧，给战斗增加高光时刻。这是三阶段单机改进的第一阶段（后续：怪物与Boss差异化、成长与遗物系统）。

### 关键决策
- 能量不自然回复，只通过战斗行为充能——鼓励进攻而非龟缩
- 终结技消耗全部能量（100），稀有但强力，约每 1-2 个房间释放一次
- 玩家实时选择释放哪个终结技（按 1/2/3），而非自动跟随精通路线
- 基础操作（近战/远程/冲刺）完全不消耗能量，保持手感自由
- 标记猎杀替代了最初设计的"弹幕爆发"，因为后者只是"更多子弹"缺乏独特感

### 已知问题 / 后续工作
- 引导提示系统（`TutorialFlags`）尚未实现，计划在后续迭代中加入
- 终结技解锁机制需要与成长/遗物系统联动，待第三阶段设计
- `cargo test` 33 项全部通过
- 下一步：怪物与Boss差异化设计

## 12. Boss 差异化设计——四Boss独特机制（2026-04-05）

### 改动内容
- **Floor1 Guardian**：新增 `BossDirectionalDefense` 组件——Boss 缓慢追踪玩家方向，正面受到 40% 伤害（60% 减伤），背面受到 150% 伤害；玩家需绕背击打弱点
- **Floor2 MirrorWarden**：传送时留下幻象（`MirrorDecoy`），幻象会发射弹幕但不闪白，最多同存 3 个；玩家需分辨真身
- **Floor3 TideHunter**：Phase1 零弹幕，靠锁定冲刺逼近；玩家冲刺穿越冲锋路径（技能判定）触发 `Stunned` 状态，硬直期受到 2.0x 伤害（后调整为 1.5x）
- **Floor4 CubeCore**：4 个浮动子核心（`BossSubCore`）全部存活时主体完全免疫伤害（`BossCoreShield`）；消灭全部核心后主体变脆，之后阶段重生 2 个更强核心
- **首次遭遇提示**：复用 `TutorialFlags` 系统，每个 Boss 首次出现时在屏幕顶部显示机制提示（"攻击背部造成更多伤害"等）
- **配置扩展**：`boss.ron` 新增每层 Boss 独立的 `phase_thresholds`、`projectile_speed`、`contact_damage` 字段

### 目的与动机
原来 4 个 Boss 体型/移动/攻击模式高度相似，只有 HP 差异，缺乏辨识度和记忆点。参考 Hades（机制奖励）、Hollow Knight（阶段变化）、Isaac（特殊互动）的设计原则，给每个 Boss 引入一个"需要理解才能高效打的核心机制"。

### 关键决策
- Guardian 使用朝向判定而非位置绕背，因为 Boss 会缓慢旋转——这让"绕到背后"有一定时间窗口而不是即时反应
- TideHunter 的反制触发设计为"冲刺穿越冲锋路径"而非"挡刀"，因为检测两个运动体碰撞比检测输入时机更可靠，且有更好的视觉反馈
- CubeCore 核心数量选 4 而非 2/6：2 个清理太快缺乏层次感，6 个让玩家感到厌烦

### 已知问题 / 后续工作
- Guardian 正背面视觉上难以区分，需要添加可视化指示器（下一次迭代修复）
- `cargo test` 33 项全部通过

## 13. Guardian 盾牌可视化 + 转向速度修正（2026-04-05）

### 改动内容
- 新增 `GuardianShieldIndicator` 标记组件，Guardian 生成时同步生成橙色矩形子实体（宽 10px、高 52px）作为盾牌方向指示器
- 盾牌指示器随 Boss 朝向实时更新位置和旋转（始终指向面朝方向前方 40px）
- Guardian 转向速度大幅降低：每帧 lerp 系数 `0.06 → 0.012`，让玩家有足够时间绕背
- `boss_guardian_facing_system` 重构：移除 Sprite 查询（导致 Query 冲突），改为通过 `Children` + `GuardianShieldIndicator` 查询更新子实体变换

### 目的与动机
测试时发现 Guardian 正背面肉眼几乎无法区分（都是白色方块），玩家无法判断当前攻击面，机制设计失效。同时原转向速度过快，绕背后 Boss 几乎立即转回来，机制窗口极短。

### 关键决策
- 用子实体而非修改 Boss 本体颜色来显示朝向，避免与 Boss 受击闪白（`DamageFlash`）逻辑冲突
- 橙色（SRGBA 1.0, 0.55, 0.1, 0.85）选择有足够视觉对比度，且不会被误认为血量/状态条

## 14. 全局平衡调整第三阶段（2026-04-06）

### 改动内容
- **Boss HP 重平衡**：Floor1~4 的 max_hp 从 300/340/360/800 调整为 330/450/600/620；打斗时长目标 25-35/35-50/50-70/70-100 秒梯级增长
- **TideHunter 硬直增伤修正**：`amount *= 2.0 → 1.5`，防止机制窗口反而成为秒杀工具（玩家 MarkedHunt 技能在 floor3 约 ATK×8 × 27 = 216 伤害，叠加硬直 2x 可一套带走 360HP 的 Boss）
- **CubeCore 有效HP修正**：初始核心 HP 从 70×4=280 降至 40×4=160；阶段重生公式 `70+phase×20 → 40+phase×10`；CubeCore 总有效 HP 从 ~1560 降至 ~1040（TideHunter 的 1.73x，合理）
- **金币奖励递增修正**：普通敌人掉落从 floor3+ 的 7 金修正为 8/10/13/16（随层递增），修复后期购买力反而下降的问题
- **玩家能量微调**：`kill_charge_gain` 12→10，`elite_kill_charge_gain` 25→22，减少 Boss 前蓄满优势
- **小怪调整**：狙击手伤害 20→17（降低惊吓死亡概率），侧翼兵速度 208→195
- **SupportCaster 重设计**：HP 54→38（必须优先击杀），增益统一为速度×1.5 + 攻速+40%（`cooldown_mult: 1.67`），覆盖目标 2→3 个，新增向玩家发射慢速弹幕

### 目的与动机
试玩发现数值曲线失控：TideHunter（floor3）被一套技能秒杀，CubeCore（floor4）却因隐性有效 HP 极高而过难；金币经济后期反而变少但商店更贵；SupportCaster 不会攻击且多了反而占用刷怪槽位减少玩家压力。参考 Hades/Isaac/RoR2 的设计原则进行系统性修正。

### 关键决策
- CubeCore 降低本体 HP 同时降低核心 HP，而非只降本体——因为核心提供的是"免疫门"，玩家每次需要完整清理，有效 HP 必须单独核算
- SupportCaster 的 `cooldown_mult` 值需要与代码公式一致：代码为 `base_cooldown / cooldown_mult`，所以"加速40%"应填 `1.67` 而非 `0.60`（Codex 初次实现错误，已修正）
- 金币奖励改为硬编码递增，暂不移入配置文件（可作为后续技术债迁移）

### 已知问题 / 后续工作
- 金币奖励仍硬编码在 `systems.rs`，后续可迁移至 `game_balance.ron`
- 数值调整需要实际游玩验证，可能需要进一步微调
- `cargo test` 33 项全部通过
- 下一步：成长与遗物系统设计

## 15. 铭文系统+诅咒系统+祝福祠堂骨架（2026-04-06）

### 改动内容
- **铭文模块** (`src/gameplay/rune/`)：`RuneSlot`（近战/远程/冲刺/终结技 4 槽位）、`RuneTier`（普通/精英/传说 3 等级）、`RuneId`（30 个铭文枚举）、`RuneLoadout` Component（每槽装 1 个铭文，装新替旧）
- **诅咒模块** (`src/gameplay/curse/`)：`CurseId`（脆弱/迟缓/枯竭/暴露/虚弱 5 种）、`CurseState` Component（诅咒生命周期管理，每进入新房间递减，到期自动消除）
- **配置文件**：`runes.ron`（30 个铭文的名称、描述、取舍、价格）、`curses.ron`（5 种诅咒的名称、效果、持续时间）
- **祝福祠堂流程**：原 Reward 房在 Floor 2+ 触发 Blessing 模式，展示 2 个铭文+诅咒选项，玩家选择后铭文装入槽位+诅咒生效，或选择离开不拿
- **祝福房规则**：每层最多 1 个，Floor 1 不出现，有诅咒时不出现新的祝福房
- **HUD 显示**：技能栏旁显示 4 个铭文槽位（空槽灰色/已装备着色），HP 条下显示诅咒状态和剩余房间数
- **配置解析**：`GameDataRegistry` 新增 `runes` 和 `curses` 字段
- **Player spawn**：挂载 `RuneLoadout::default()` 和 `CurseState::default()`

### 目的与动机
当前奖励系统只有属性加成（+ATK/+HP 等），所有获取途径给的都是同质化数值提升，缺乏构建感。引入双轨系统：属性成长（保留现有，稳定变强）+ 铭文系统（改变玩法，构建差异化）。参考 Hades（Boon + 混沌之门）、Slay the Spire（卡牌 + 遗物）、Isaac（道具 + 恶魔房）的设计。

### 关键决策
- 铭文是"替换"而非"叠加"——4 个槽位永远只有 4 个铭文，迫使玩家做选择而非无限变强
- 祝福祠堂采用 Hades 混沌之门模式：强力铭文必须承受临时诅咒，诅咒未消除时不会出现新的祝福房（防止叠加失控）
- 本步只做数据流通（选择→装备→显示），不实现铭文战斗效果——降低单次改动风险，效果实现在后续迭代
- 复用 `AppState::RewardSelect` 而非新增状态，减少状态机复杂度

### 已知问题 / 后续工作
- 铭文战斗效果未实现（第三步）
- 事件房重设计未实现（第二步）
- 商店铭文商品未实现（第二步）
- Boss 奖励调整未实现（第二步）
- Coop 模式下祝福祠堂未同步（需单独工作）
- `cargo test` 33 项全部通过
- 完整设计文档：`docs/superpowers/specs/2026-04-06-reward-system-redesign.md`

## 16. TideHunter 重设计为"影子猎人" + HUD B0001 修复（2026-04-06）

### 改动内容
- **TideHunter 状态机重写**：从简单的"靠近→蓄力→冲刺→冷却"改为"Stalk→Telegraph→ShadowDash→Reposition"循环，核心机制是快速穿越留下紫色影子轨迹（持续伤害地带）
- **影子轨迹系统**：新增 `ShadowTrail` 组件，Boss 穿越路径每 25px 生成一个影子实体，半透明深紫色，线性淡出后 despawn；站在影子上持续受伤，冲刺穿越免疫
- **三阶段递进**：Phase1 单次穿越/影子 2.5s/停顿 0.9s；Phase2 连续 2 次穿越/影子 3.5s/停顿 0.7s + 1 颗弹；Phase3 连续 3 次穿越（三角形包围）/影子 4.5s/停顿 0.6s + 3 颗扇形弹
- **反制机制调整**：从 WindupTelegraph 阶段改为 Reposition 阶段检测——玩家需要穿越影子接近 Boss 才能触发 Stunned（1.4s，伤害 ×1.5）
- **HUD B0001 修复**：`update_rune_and_curse_ui` 的两个 `Text` Query 加 `Without<>` 过滤，解决运行时 panic

### 目的与动机
试玩反馈第三层 Boss 设计单调：循环太简单，只有一个交互机制（冲刺打断），阶段差异仅为攻速变化。重设计为"影子猎人"风格，让战场空间随阶段推进被逐渐压缩，玩家需要在影子缝隙中走位并找准反击窗口。B0001 是铭文系统引入后的 Bevy Query 冲突 bug。

### 关键决策
- 影子轨迹是持续伤害而非一次性伤害——鼓励玩家主动走位而非站桩输出
- 反制窗口从蓄力阶段移到停顿阶段——玩家需要穿越危险的影子区域才能反击，风险与回报对等
- Phase3 三角形穿越路径——让战场被三条影子线切割，空间极度压缩，体现最终阶段的压迫感

### 已知问题 / 后续工作
- 需要实际游玩验证影子伤害数值和持续时间是否合理
- `cargo test` 33 项全部通过

## 17. 奖励系统重构——强化数据模型 + XP/升级系统骨架（2026-04-06）

### 改动内容
- **强化模块** (`src/gameplay/augment/`)：`AugmentId`（30 个强化枚举）、`AugmentRarity`（普通/精英/传说）、`AugmentCategory`（近战/远程/机动/通用）、`HeldAugment`（含叠加层数）、`AugmentInventory` Component（自由收集，无槽位限制）
- **经验系统** (`src/gameplay/progression/experience.rs`)：`PlayerLevel` Component（等级/XP/升级阈值）、`XpGainEvent`/`LevelUpEvent` 事件、`process_xp_gains` 系统
- **配置文件** `assets/configs/augments.ron`：30 个强化的完整定义（id、类别、稀有度、名称、描述、升级描述、商店价格）
- **数据注册**：`AugmentConfig`/`AugmentsConfig` 接入 `GameDataRegistry`，加载器支持 `augments.ron`
- **玩家 spawn**：`spawn_player` 挂载 `AugmentInventory::default()` + `PlayerLevel::default()`

### 目的与动机
试玩反馈：奖励房太少（4 层只见 1 个）、成长存在感低、所有成长走同一管道（三选一）缺乏构建感。决定将成长拆为两层独立系统：
- 第 1 层：属性成长（经验升级 + 简单属性选择，确定性）
- 第 2 层：强化构建（Augment 系统，随机性，替代旧铭文+精通）

本次为阶段 1，只建立数据骨架和事件管线，不实现战斗效果。

### 关键决策
- 强化系统替代旧铭文系统（RuneLoadout 4 槽位 → AugmentInventory 无槽位限制），旧铭文模块暂时保留以避免大规模破坏性重构
- 同类强化可升级（stacks max 2），而非无限叠加
- XP 阈值公式 `40 + (level-1) × 15`，保证前期升级快、后期放缓
- augments.ron 使用 `unwrap_or` 容错加载，缺失时不阻塞游戏启动

### 已知问题 / 后续工作
- 阶段 2：强化获取流程 + 升级选择 UI（AugmentSelect/LevelUpSelect 状态）
- 阶段 3：30 个强化的战斗效果实现
- 阶段 4：事件房 + 商店扩展 + 祝福祠堂改造
- 阶段 5：新怪物（Bomber/Shielder/Summoner）+ 精英词缀 + TideHunter 调整
- 阶段 6：HUD + 平衡 + 旧代码清理
- 完整设计文档：`PLANS.md`
- `cargo test` 33 项全部通过

## 2026-04-06 游戏表现力增强：音效 + 视觉特效 + UI 打磨

### 改动内容

**Phase 1 — 基础设施 + 核心音效 + 核心特效：**
- 新增事件：`SfxEvent`（13 种音效类型）、`HitStopRequest`、`ScreenFlashRequest`
- 新增缓动函数：`ease_out_cubic`、`ease_out_expo`、`ease_out_elastic`、`ease_out_back`
- 新增配置：`audio.ron`（音量/pitch 变化）、`effects.ron`（粒子数/打击暂停/闪光参数）
- 重写 `src/core/audio.rs`：程序化波形合成生成 13 种音效，WAV 编码后插入 `Assets<AudioSource>`，事件驱动播放
- 新建 `hitstop.rs`：基于 `Time<Virtual>` 时间缩放的打击暂停系统
- 新建 `screen_flash.rs`：全屏半透明覆盖 + `ease_out_expo` 衰减
- 新建 `death_effect.rs`：敌人死亡粒子爆炸（普通 16 / Boss 32 粒子 + 闪光 + 震动）
- 桥接系统复用已有事件，直接集成近战/远程/冲刺音效

**Phase 2 — UI 动画 + 增强粒子：**
- 血条/能量条平滑 lerp 动画（`BarAnimState`，指数衰减插值）
- 粒子增强：随机大小、角度偏移、速度衰减、辉光模拟层
- 技能槽位能量不足时渐变灰暗
- 奖励卡片弹入动画（`CardAnim`，`ease_out_back` 缩放）

**Phase 3 — 打磨 + BGM 框架：**
- Boss 阶段切换增强：屏幕闪光 + 震动 + 打击暂停
- BGM 状态机框架：`BgmState` 资源，根据游戏状态自动切换曲目类型

### 目的与动机
游戏手感基础不错但存在零音频、视觉反馈缺失、UI 缺乏动感三个短板。程序化音效生成避免外部资源依赖，保持几何风格一致性。

### 关键决策
- 程序化波形合成：零依赖、即时可用、风格统一
- 桥接系统模式：复用已有事件，避免侵入式修改大量系统签名
- `Time<Virtual>` 时间缩放实现打击暂停
- 配置驱动：粒子数、暂停时长等均从 `effects.ron` 读取

### 已知问题 / 后续工作
- BGM 框架已就绪但无实际音频播放（需外部 .ogg 或程序化环境音）
- 连击计数器 UI 未实现（当前无 ComboCounter 系统）
- `cargo test` 33 项全部通过

---

## 2026-04-06 强化系统阶段2——XP/升级 + 强化选择 + 奖励流程简化

### 改动内容

**阶段1（数据骨架，前次提交已完成）：**
- 新增 `AugmentId`（30个强化）、`AugmentInventory` 组件、`PlayerLevel` 组件
- `assets/configs/augments.ron` 定义全部30个强化的稀有度、类别、描述
- `GameDataRegistry` 新增 `augments` 字段

**阶段2（本次提交）：**
- 敌人死亡发送 `XpGainEvent`（普通怪 8-15 XP，精英 25-40，Boss 100+）
- `process_xp_gains` 系统处理 XP 累积和升级，支持 `XpBonus` 强化加成
- `handle_levelup_event` 系统：升级时生成3个随机属性选项，进入 `LevelUpSelect` 状态
- 新增 `AugmentSelect` / `LevelUpSelect` 两个 `AppState`，完整 UI 生命周期
- 奖励流程简化：普通房通关不再弹 `RewardSelect`，仅40%概率弹 `AugmentSelect`
- Boss房：治疗80% → 必定 `AugmentSelect` → `RewardSelect`（楼层转换）
- `GoldBonus` / `XpBonus` 强化效果已接入战斗系统
- 部分 Common 强化效果已实现（SpeedBoost、HeavyStrike、DashTrail、ExtendedInvuln 等）
- `DashEnergy` 系统：冲刺穿敌回复能量

### 目的与动机
试玩反馈：奖励房太少、成长管道单一、每房三选一打断节奏。将成长拆为两层独立系统（属性升级 + 强化收集），大幅增加获取频率和构建多样性。普通房通关改为即时反馈（金币+XP已在击杀时获得），选择界面只在概率触发或Boss时出现。

### 关键决策
- 普通房不再弹 RewardSelect：减少选择疲劳，XP/金币已在击杀时即时获得
- 强化掉落概率40%/100%：保证每局能收集到足够强化，但不是每房都打断
- `AugmentSelect` 返回状态可配置：普通房返回 `InGame`，Boss房返回 `RewardSelect`
- Codex 调用方式确认：`codex-companion.mjs task --write` 可从 Claude 内部写代码

### 已知问题 / 后续工作
- 阶段3：30个强化的战斗效果实现（大部分尚未接入）
- 阶段4：事件房、商店扩展、掉落物系统
- 阶段5：新怪物（Bomber/Shielder/Summoner）+ 精英词缀
- 阶段6：HUD 更新 + 数值平衡 + 清理旧铭文代码
- `cargo check` 通过，`cargo test` 33 项全部通过
