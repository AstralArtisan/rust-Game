# 项目总览与合作联机审查

- 适用版本：历史评审快照，不作为当前唯一基线
- 最后校验：2026-03-31；仅补充历史文档标识，正文保留原始评审内容
- 关联源码：`src/`、`docs/00_index.md`、`docs/06_multiplayer_and_risks.md`
- 实验性内容：是。本文保留了阶段性评审结论

> 历史资料说明
>
> 本文是项目中期总览与 Coop 评审记录，许多内容仍有参考价值，但它不是正式维护手册。当前工程交接请优先从 `docs/00_index.md` 开始，并以新的分层文档为准。

更新时间：2026-03-27  
审查基线（2026-03-27 当时）：当前工作树源码 + `cargo check` 通过 + `cargo test` 通过（当时为 `0 tests`）  
审查范围：总结项目整体内容，重点分析 `Coop` 合作联机；`PVP` 只做架构和输入层问题的简要对照。

## 1. 项目概览

`Block City Adventure / 勇闯方块城` 是一个基于 Rust + Bevy 的 2D 俯视角动作 Roguelike 课程项目。项目已经形成“单机可玩 + 合作联机原型 + PVP 原型”的基本结构，代码组织以 Bevy 插件和功能模块拆分为主。

- 技术栈：
  - Rust 2024
  - Bevy 0.14
  - `bevy_rapier2d` 0.27
  - `bevy_kira_audio` 0.20
  - `serde + ron`
  - `bincode`
  - `lightyear` 0.17.1
- 主要模块分层：
  - `src/core/`：输入、资源、音频、相机、存档、成就、本地联调入口
  - `src/data/`：配置定义、加载器、全局注册表
  - `src/gameplay/`：地图、玩家、战斗、敌人、奖励、商店、解谜、成长
  - `src/coop/`：合作联机组件、网络层、权威运行时、联机 UI
  - `src/pvp/`：PVP 网络、对战运行时、对战 UI
  - `src/ui/`：主菜单、HUD、暂停、奖励、商店、结算
- 主要状态流：
  - 单机：`MainMenu -> InGame -> RewardSelect / Shop / Paused / GameOver / Victory`
  - 合作：`MainMenu -> MultiplayerMenu -> CoopMenu -> CoopLobby -> CoopGame`
  - PVP：`MainMenu -> MultiplayerMenu -> PvpMenu -> PvpLobby -> PvpGame -> PvpResult`
- 数据驱动方式：
  - 主要数值来自 `assets/configs/*.ron`
  - 玩家、敌人、Boss、奖励、房间和平衡参数都通过配置加载到 `GameDataRegistry`

从代码形态看，项目已经不是简单 demo，而是一个具备完整玩法骨架的练习型游戏工程：主循环、敌人 AI、奖励成长、Boss、商店、成就、存档、多人原型都已经存在，只是联机部分仍处于“原型整合期”。

## 2. 单机部分

单机主循环仍然是项目最完整、最稳定的部分。

- 入口与初始化：
  - `src/main.rs` 创建 Bevy 应用
  - `src/app.rs` 注册 `GameplayPlugin`、`UiPlugin`、`SavePlugin`、`AchievementsPlugin` 等
- 地图与房间推进：
  - `src/gameplay/map/generator.rs` 负责楼层与房间布局生成
  - 核心资源包括 `FloorLayout`、`CurrentRoom`、`RoomState`、`FloorNumber`、`VisitedRooms`
  - 单机房间切换由 `src/gameplay/map/transitions.rs` 的 `RoomTransition` 负责
- 房间类型：
  - `Start`
  - `Normal`
  - `Reward`
  - `Shop`
  - `Puzzle`
  - `Boss`
- 玩家与战斗：
  - `src/gameplay/player/` 负责移动、朝向、近战、远程、冲刺、连击、动画
  - `src/gameplay/combat/` 负责 hitbox/hurtbox、伤害、击退、DOT、投射物
  - `src/gameplay/effects/` 负责闪白、拖影、伤害数字、粒子等表现
- 敌人与 Boss：
  - `src/gameplay/enemy/ai.rs` 和 `src/gameplay/enemy/systems.rs` 负责刷怪、AI、攻击、死亡、Boss 行为
  - 敌人种类包含追击、远程、冲锋、侧袭、狙击、辅助、Boss
- 奖励、商店、成长：
  - `src/gameplay/rewards/systems.rs` 负责清房奖励和奖励房奖励
  - `src/gameplay/shop/mod.rs` 负责商店商品和购买
  - `src/gameplay/progression/` 负责楼层难度和进度资源
- 存档与成就：
  - `src/core/save.rs` 与 `src/core/achievements.rs` 已接入全局插件

整体上，单机模式已经形成“进房 -> 战斗/事件 -> 奖励/商店 -> Boss -> 下一层”的完整闭环。相比联机，单机的状态、资源和世界实体都在同一个本地世界里，逻辑边界更清晰，问题也更少。

## 3. 联机部分

项目的联机部分分成两条线：`Coop` 合作模式和 `PVP` 对战模式，两者没有共用同一套网络实现。

### 3.1 Coop：Lightyear + 主机权威

合作模式的核心特征是“主机权威模拟”，不是双端对等模拟。

- `src/coop/net.rs`
  - 使用 `Lightyear` 注册 client/server 插件
  - Host 在同一进程内同时跑本地 client 和 server
  - 复制的核心组件包括玩家、敌人、投射物、门和 `CoopSessionState`
- `src/coop/runtime.rs`
  - Host 负责真正生成楼层、玩家、会话实体
  - Host 负责权威推进玩家输入、敌人行为、奖励、商店、选门、RPS、结算
- `src/coop/ui.rs`
  - 客户端和 Host 本地画面主要依赖复制体可视化
  - 联机 HUD、暂停覆盖层、奖励/RPS/商店弹层都在这里

当前 Coop 的设计要点可以概括为：

- Host 执行玩法模拟
- Client 上传输入并消费复制结果
- `CoopSessionState` 用来同步联机阶段、房间状态、奖励/商店/路线选择信息
- 门选择当前以“走到真实门旁按 `E`”为主，不是完整的消息驱动 UI 流

### 3.2 PVP：手写 UDP

PVP 是另一套更轻量的网络实现。

- `src/pvp/net.rs`
  - 基于 `UdpSocket`
  - 使用 `Hello / Welcome / Input / State / Fire / Result` 消息
- `src/pvp/systems.rs`
  - Host 做对战模拟
  - Client 做本地预测和状态应用
- `src/pvp/ui.rs`
  - 负责房主/加入菜单、对战大厅和结果页

和 Coop 相比，PVP 的玩法更简单，网络实现也更直接；但加入地址输入层的问题与 Coop 有共性。

### 3.3 本地联调入口

README 仍然写着 `local_debug/*.ps1` 脚本，但当前仓库的真实入口是 `src/core/local_debug.rs` 的 `LocalDebugPlugin`，通过环境变量驱动。

实际支持的本地联调变量包括：

- `LOCAL_NET_DEBUG`
- `LOCAL_NET_DEBUG_MODE`
- `LOCAL_NET_DEBUG_ROLE`
- `LOCAL_NET_DEBUG_HOST`

这意味着多人调试入口已经从“脚本驱动”改成了“环境变量驱动”，文档需要同步。

## 4. 合作联机问题清单

本节只列当前代码已经能直接确认的问题；不直接沿用 `docs/coop_network_audit.md` 中已经失效的历史结论。

### 4.1 当前代码已确认的问题

| 优先级 | 问题 | 当前确认 | 影响 |
| --- | --- | --- | --- |
| P0 | 会话退出流程不完整 | `src/coop/ui.rs` 中“返回大厅”只做 `next_state.set(AppState::CoopLobby)`，没有 teardown 联机会话、运行时和连接 | 玩家从暂停面板返回大厅后，`src/coop/net.rs` 的 `auto_advance_lobby_state()` 可能立刻把双方重新送回 `CoopGame`，导致“看起来返回大厅，实际上没有离场” |
| P0 | 客户端断线后不会主动退场 | `src/coop/net.rs` 的客户端断线事件只把 `connected/local_connected` 置为 `false`，没有把 `CoopGame` 切回大厅或主菜单 | Host 退出、崩溃或网络断开时，客户端可能停留在 `CoopGame` 的冻结世界里，只是网络标志变了 |
| P1 | 地址输入允许填 `IP:端口`，但连接构造仍会二次拼端口 | Coop 和 PVP 的加入输入都允许 `:`；Coop 的 `build_lightyear_client_net_config()` 使用 `format!("{host_ip}:{COOP_PORT}")`，PVP 的加入逻辑使用 `format!("{host}:{PVP_PORT}")` | 如果用户输入 `192.168.1.6:3457`，Coop 解析失败后会静默回退到 `127.0.0.1:3457`，PVP 则拿不到有效 `peer`，都属于高误导性输入问题 |
| P1 | 协议定义与实际运行时不一致 | `CoopNetState.peer` 在 Coop 中被写入，但 Lightyear 实际连接目标来自 `CoopNetConfig.host_ip`；`CoopCommandMessage` 里定义了 `LeaveSession` 和 `SelectDoor`，但当前 UI/运行时没有完整走通 | 排查联机问题时容易误判真实连接源，也容易误以为“选门是消息协议驱动的”，但当前实际逻辑仍是“走到门边按 `E`” |
| P2 | 联机文档与调试入口漂移 | README 仍引用不存在的 `local_debug/*.ps1` 脚本；真实入口已经变成 `src/core/local_debug.rs` | 新成员或排障人员会被过时文档误导，增加联调成本 |

对“协议定义与实际运行时不一致”需要额外说明两点：

- `SelectDoor`
  - 协议和 Host 消费分支存在
  - 但 `src/coop/ui.rs` 在 `CoopPhase::DoorChoice` 下没有发送该命令
  - 当前实际选门逻辑由 `src/coop/runtime.rs` 的 `host_handle_door_interactions()` 读取玩家站位和 `interact_pressed`
- `LeaveSession`
  - 只定义在 `CoopCommandMessage` 中
  - 当前没有对应的发送逻辑，也没有 Host 侧消费逻辑

### 4.2 旧审计文档中已失效的判断

以下结论出现在 `docs/coop_network_audit.md` 中，但以当前源码为准，它们已经失效或至少需要降级处理。

1. “单机奖励系统会在 `CoopGame` 中抢状态，把主机切到 `RewardSelect`”  
   这一结论已经失效。当前 `src/gameplay/rewards/systems.rs` 明确只在 `AppState::InGame` 下运行，不会直接抢占 `CoopGame`。

2. “Coop 仍会生成 `Puzzle` 房，但 `PuzzlePlugin` 不在 `CoopGame` 运行”  
   这一结论已经失效。当前 `src/coop/runtime.rs` 和 `src/gameplay/enemy/systems.rs` 都会把 Coop 下的 `RoomType::Puzzle` 规范化为 `RoomType::Normal`，不再保留原来的联机谜题房路径。

3. “旧审计中点名的多个 `Replicated` 污染点依然成立”  
   这一结论需要降级。当前至少以下关键链路已经明确排除了 `Replicated`：
   - `src/gameplay/combat/projectiles.rs` 的投射物移动与出界销毁
   - `src/gameplay/enemy/ai.rs` 的敌人 AI 快照
   - `src/gameplay/enemy/systems.rs` 的敌人攻击、敌人死亡计数

4. “客户端只要 `connected == true` 就会进入 `CoopGame`”  
   这一结论也已失效。当前 `src/coop/net.rs` 的 `auto_advance_lobby_state()` 对 Client 还要求联机会话实体、可控本地复制玩家、复制门都已就绪，才会进入 `CoopGame`。

需要强调的是：旧审计文档并不是完全不可用，而是部分结论已经落后于当前代码。继续直接沿用，会把排查重点带偏。

## 5. 合作联机修改建议

以下建议按“短期可落地修复”的优先级排序，目标是用 1 个迭代把最影响联调和联机体验的问题先压下去。

### 5.1 建议一：补齐统一的会话生命周期 teardown

优先级：最高

建议把“离开合作会话”做成统一入口，而不是继续只改 `AppState`。

建议方向：

- 为 Coop 增加统一的离场函数或系统入口，例如“leave/reset current coop session”
- 统一处理以下动作：
  - 断开 client/server
  - 清空 `CoopRuntimeState`
  - 清空 `CoopNetState`
  - 清理权威实体与复制实体
  - 最后再切换到 `CoopLobby` 或 `MainMenu`
- `src/coop/ui.rs` 的“返回大厅”
  - 不应只做 `next_state.set(AppState::CoopLobby)`
  - 应先触发 teardown，再进入大厅
- `src/coop/runtime.rs` 的断线回退
  - 也应走同一条 teardown 路径，避免 Host 和 Client 的退场逻辑分叉

推荐接口决策：

- `CoopCommandMessage::LeaveSession`
  - 建议补齐，而不是继续保留死分支
  - 如果短期不想走网络消息，也至少要删除该协议分支，改成明确的本地 teardown 事件

### 5.2 建议二：补齐客户端断线恢复与用户提示

优先级：高

当前客户端断线后只是网络标记变化，没有应用状态变化，这会直接制造“画面还在，联机已死”的假在线状态。

建议方向：

- 在 `src/coop/net.rs` 的客户端断线事件处理中：
  - 如果当前在 `CoopGame`
  - 主动切回 `CoopLobby` 或 `MainMenu`
  - 同时推送一条明确提示，例如“与房主断开连接，已退出合作会话”
- Host 侧如果远端掉线，也不要只依赖 `host_cleanup_disconnected_session()` 改状态
  - 同样应走统一 teardown

推荐默认行为：

- 开发调试期优先退回 `CoopLobby`
- 正式体验期更适合退回 `MainMenu` 并提示断线原因

### 5.3 建议三：收紧加入地址的输入与解析规则

优先级：高

当前的地址输入问题属于“看上去允许，实际上解析不对”的高误导设计，应该尽快修。

建议方向二选一，但短期建议直接选第一种：

1. 只接受裸主机地址，不接受 `host:port`
   - 输入框只允许 `IPv4`
   - UI 明确提示端口固定为 `3457` / `3456`
   - 用户输入 `:` 时直接报错或忽略
2. 升级为显式解析 `SocketAddr`
   - 真正允许 `host:port`
   - 解析失败时提示错误
   - 不允许静默回退 localhost

推荐接口决策：

- `CoopNetConfig.host_ip`
  - 若继续保留当前字段名，语义必须明确为“裸主机地址，不含端口”
  - 更彻底的做法是升级为显式 `SocketAddr`
- `CoopNetState.peer`
  - 若继续保留，必须降级为“仅调试显示字段”
  - 否则建议直接移除，避免与真实连接源混淆

### 5.4 建议四：清理死协议分支，统一门选择语义

优先级：中

当前门选择的真实逻辑是“靠近房门按 `E`”，而不是“通过消息发送选门命令”。协议定义和真实玩法已经分叉。

建议方向：

- `CoopCommandMessage::SelectDoor`
  - 短期建议删除
  - 因为当前 UI 没有配套命令发送，运行时也不是靠它推进
- `DoorChoice` 阶段
  - 文档和 UI 提示明确写成“走到真实门旁按 `E` 锁定路线”
  - 不要继续保留看似完整、实际上未走通的命令式接口

如果未来要做纯 UI 式路线投票，再重新引入消息驱动的选门协议会更干净。

### 5.5 建议五：修正文档与联调入口说明

优先级：中

建议同步修正以下文档：

- `README.md`
  - 去掉不存在的 `local_debug/*.ps1`
  - 改成 `LocalDebugPlugin + 环境变量` 的真实入口
- `docs/coop_network_audit.md`
  - 标出“历史审计，部分结论已过时”
  - 删除或降级已经失效的奖励、谜题、`Replicated` 旧结论

这类修正不解决玩法 bug，但能显著减少后续排查成本。

### 5.6 最小验证矩阵

当前仓库可直接执行的最小验证矩阵如下：

| 场景 | 目标 | 当前结论 |
| --- | --- | --- |
| `cargo check` | 确认当前工作树可通过编译检查 | 已通过 |
| `cargo test` | 确认当前工程测试入口可运行 | 已通过，但当前为 `0 tests`，属于明显质量缺口 |
| 本机双开 Coop | 验证创建房间、加入、进入游戏、暂停后返回大厅 | 建议作为修复会话生命周期后的第一验证项 |
| 两机局域网 Coop | 验证裸 IP 可连、`IP:端口` 会失败、主机断开后客户端是否正确退场 | 目前应重点关注地址输入和断线恢复 |
| PVP 加入页 | 验证同类 `IP:端口` 输入问题是否存在 | 当前也存在，属于多人输入层共性问题 |

结论上，当前 Coop 更像“已经有完整玩法骨架，但会话生命周期和联调入口还不稳定”的原型系统。短期最值得先修的不是更多玩法，而是：

1. 会话退出与断线恢复
2. 地址输入与解析
3. 死协议分支清理
4. 文档与调试入口同步

这四项修完之后，合作联机的可调试性和可维护性会明显提升。
