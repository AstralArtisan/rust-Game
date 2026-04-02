# Coop 联机审计文档

- 适用版本：历史审计快照，不作为当前唯一基线
- 最后校验：2026-03-31；仅补充历史文档标识，正文保留原始审计内容
- 关联源码：`src/coop/`、`src/gameplay/`、`docs/06_multiplayer_and_risks.md`
- 实验性内容：是。本文反映的是审计当时的观察结论

> 历史资料说明
>
> 本文是早期 Coop 审计记录，部分结论已经随源码演进而过时。阅读当前实现时，应优先参考 `docs/06_multiplayer_and_risks.md`、`docs/02_architecture.md` 和 `docs/03_module_design.md`，再把本文作为背景资料对照使用。

更新时间：2026-03-26  
审计基线：当前工作树（包含未提交改动），`cargo check` 通过  
审计范围：只看 `Coop` 联机链路；`PVP` 只作为对照，不展开

## 1. 审计结论

- 当前合作联机不是“双端对等模拟”，而是“主机权威模拟 + Lightyear 复制 + 客户端局部显示/UI”。
- 主机进程会同时启动 `server` 和本地 `client`。这意味着主机世界里可能同时存在：
  - 权威实体：真正参与玩法模拟。
  - 复制实体：由 Lightyear 回流，用于主机本地显示。
- 只要某些玩法系统没有一致地排除 `Replicated`，就会出现“主机看到的世界”和“客户端看到的世界”不一致，甚至主机自己也会被复制体污染。
- `Coop` 没有完全隔离单机玩法分支，当前至少有两类高风险冲突：
  - 单机奖励系统仍然全局运行，可能在 `CoopGame` 中抢状态，直接把主机切到 `RewardSelect`。
  - 地图生成仍然会产出 `Puzzle` 房，但 `PuzzlePlugin` 只在 `InGame` 运行，`CoopGame` 下没有对应逻辑，且谜题实体也未复制到客户端。
- 客户端主要依赖复制后的 `CoopSessionState`、玩家/敌人/门/投射物组件来展示。许多单机资源并不会复制，因此 HUD、房间信息、楼层信息、房间装饰和部分世界表现天然容易与主机不一致。

## 2. 一页总览

### 2.1 插件与状态总览

```text
main.rs
  -> GamePlugin
     -> GameplayPlugin
     -> CoopPlugin
        -> CoopLightyearPlugin
           -> Lightyear client plugins
           -> Lightyear server plugins
           -> InputPlugin<CoopInputState>
           -> Coop protocol registration
        -> CoopRuntimePlugin
           -> 主机权威模拟、会话推进、房间/奖励/商店/选门/RPS
        -> Coop UI systems
           -> 菜单、大厅、联机 HUD、复制实体可视化、本地暂停覆盖层
```

`Coop` 使用的核心状态流：

```text
MainMenu
  -> MultiplayerMenu
  -> CoopMenu
  -> CoopLobby
  -> CoopGame
  -> MainMenu
```

### 2.2 Host / Client 状态流

**Host**

1. 在 `CoopMenu` 按 `H`。
2. `CoopNetConfig.mode = Host`，`start_host_socket()` 只初始化状态，不真正启动网络。
3. 进入 `CoopLobby` 后，`sync_coop_network_lifecycle()` 启动：
   - Lightyear server，监听 `0.0.0.0:3457`
   - 主机本地 Lightyear client，连接 `127.0.0.1:3457`
4. `local_connected && remote_connected` 后，大厅自动切到 `CoopGame`。
5. `host_bootstrap_match()` 在主机侧真正生成楼层、玩家、会话实体，并开始权威模拟。

**Client**

1. 在 `CoopMenu` 手输 IP，按 `J/Enter`。
2. `CoopNetConfig.mode = Client`，`host_ip` 记录输入值，`start_client_socket()` 只初始化状态。
3. 进入 `CoopLobby` 后，`sync_coop_network_lifecycle()` 用 `config.host_ip` 创建 Lightyear client 配置并发起连接。
4. 只要 `connected == true`，大厅就会切到 `CoopGame`。
5. 进入 `CoopGame` 不代表会话实体、玩家复制体、门、敌人、投射物已经全部到齐。

### 2.3 Coop 复用与改写边界

| 模块 | Coop 是否复用 | 现状 |
| --- | --- | --- |
| 玩家移动/朝向/攻击/冲刺/敌人 AI/伤害结算 | 复用 | 在 `CoopGame` 下只应由主机权威运行 |
| 房间切换 | 改写 | 单机 `RoomTransition` 不用于 Coop；改为 `host_handle_door_interactions()` + `host_tick_rps_resolution()` |
| 奖励/商店 | 改写 | Coop 自己用 `CoopSessionState` 维护阶段和选项 |
| 暂停 | 改写 | Coop 用本地覆盖层，不切到 `Paused` |
| Puzzle | 未改写完整 | 会生成谜题房，但 `PuzzlePlugin` 不在 `CoopGame` 运行，是明显缺口 |
| PVP 网络 | 不复用 | 另一套手写 UDP，与 Coop 无关 |

## 3. 网络数据面

### 3.1 连接生命周期

关键资源与常量：

- `CoopNetConfig`
  - `mode`: `None / Host / Client`
  - `host_ip`: 仅客户端和本地调试需要的目标地址
- `CoopNetState`
  - `connected`
  - `local_connected`
  - `remote_connected`
  - `server_started`
  - `client_started`
  - `local_client_id`
  - `remote_client_id`
  - `latest_inputs`
  - `received_commands`
  - `pending_commands`
- 常量
  - 端口：`3457`
  - Host client id：`1`
  - Remote client id：`2`
  - 传输：Lightyear `Netcode` + UDP

关键事实：

- `start_host_socket()` / `start_client_socket()` 只是重置状态并预设 `client_id`，真正的 server/client 启停都在 `sync_coop_network_lifecycle()`。
- Host 模式下，主机本地 client 永远连 `127.0.0.1`，这是给“同一进程内的本地 client”用的，不是给远端机器看的。
- Client 模式下，真实连接目标来自 `CoopNetConfig.host_ip`，不是 `CoopNetState.peer`。
- `CoopNetState.peer` 在 Coop 里只被写入，不参与 Lightyear 连接配置；它更像旧实现残留状态，不能当作真实连接源。

### 3.2 输入链路

```text
keyboard/mouse
  -> core::input::collect_player_input()
  -> PlayerInputState
  -> coop::net::buffer_local_inputs()
  -> LyInputManager<CoopInputState>
  -> Lightyear input stream
  -> host 侧 capture_server_inputs()
  -> CoopNetState.latest_inputs
  -> host_buffer_player_inputs()
  -> 两个权威玩家的 PlayerDriveInput
```

关键点：

- Host 的 `P1` 输入直接来自本机 `PlayerInputState`。
- Host 的 `P2` 输入来自 `latest_input_for(remote_client_id)`。
- 客户端自己不做权威玩法模拟，它只是上送输入并等待复制结果。

### 3.3 命令链路

命令协议类型：`CoopCommandMessage`

| 变体 | 当前是否走通 | 说明 |
| --- | --- | --- |
| `SelectReward` | 是 | 客户端/主机 UI 会入队，Host 在 `host_process_phase_commands()` 消费 |
| `SelectRps` | 是 | 同上 |
| `BuyShopItem` | 是 | 同上 |
| `SelectDoor` | 否 | 协议已定义，但当前 UI 没有发送；门选择实际靠 Host 读取两名玩家的交互输入与站位 |
| `LeaveSession` | 否 | 已定义，但当前没有发送也没有消费逻辑 |

命令真实路径：

```text
Coop UI 输入
  -> queue_command()
  -> CoopNetState.pending_commands
  -> flush_pending_client_commands()
  -> Lightyear message
  -> receive_coop_command_messages()
  -> CoopNetState.received_commands
  -> host_process_phase_commands()
```

结论：

- 当前“奖励 / RPS / 商店”走的是显式消息通道。
- 当前“选门”不是消息驱动，而是 Host 直接根据双方 `BufferedCoopInput + Transform` 判定。这意味着：
  - 协议里虽然有 `SelectDoor`，但它不是现网链路。
  - 远端玩家选门能否生效，本质取决于 Host 是否正确收到了远端输入并正确维护了远端权威玩家位置。

### 3.4 实体复制链路

Lightyear 当前注册复制的组件白名单：

- 玩家相关：`Player`、`Health`、`Gold`、`MoveSpeed`、`FacingDirection`、`AnimationState`
- Coop 相关：`CoopParticipant`、`PlayerSlot`、`GhostState`、`CoopNetPosition`、`CoopNetVelocity`、`CoopNetRotation`、`CoopMeleeFlashState`、`CoopDashVisualState`、`CoopSessionState`
- 世界相关：`Enemy`、`EnemyKind`、`Projectile`、`Door`

Host 复制流程：

1. `host_tag_replicated_entities()`
   - 给玩家、敌人、投射物、门补 `Replicate`
2. `host_sync_network_views()`
   - 把权威实体的 `Transform/Velocity/Facing` 写回 `CoopNetPosition/Velocity/Rotation`
3. 客户端 `attach_replicated_visuals()`
   - 给复制体补本地 `SpriteBundle/Transform/Name` 等显示组件
4. 客户端 `sync_replicated_visuals()`
   - 用 `CoopNetPosition/Rotation` 平滑驱动复制体显示
5. 客户端 `update_coop_overlay()`
   - 读取复制过来的 `CoopSessionState`

关键结论：

- `Transform`、`Sprite`、`GlobalTransform` 不是协议白名单的一部分，客户端能显示复制实体，是因为 `attach_replicated_visuals()` 在本地补了显示组件。
- 谜题实体、商店亭、命中特效、伤害数字、普通粒子等不在复制白名单里，默认不会出现在客户端。

## 4. 权威模拟与显示分层

### 4.1 主机负责什么

主机权威运行的内容包括：

- 对局启动：`host_bootstrap_match()`
- 双人输入落地：`host_buffer_player_inputs()`
- 玩家死亡/幽灵/复活：`host_handle_coop_player_deaths()`
- 房间进入与阶段推进：`host_enter_room_phase()`、`host_enter_reward_phase_on_room_clear()`
- 奖励/商店/选门/RPS：`host_process_phase_commands()`、`host_handle_shop_exit_inputs()`、`host_handle_door_interactions()`、`host_tick_rps_resolution()`
- 玩家/战斗/敌人/地图核心系统：
  - `PlayerPlugin`
  - `CombatSystemsPlugin`
  - `EnemySystemsPlugin`
  - `MapPlugin`（不含单机 `RoomTransition`）

### 4.2 客户端主要负责什么

客户端主要负责：

- 本地输入采集与上送
- 复制体显示与插值
- 本地 `LocalControlled` 标记
- 相机跟随本地玩家
- 联机 HUD 与覆盖层
- 本地暂停覆盖层（只是 UI，不切换到 `Paused`）

### 4.3 Host 为什么容易出现“自己也不一致”

`sync_host_authority_visibility()` 的目标是：

- 主机仍用权威实体做模拟
- 主机屏幕尽量显示复制体，避免“一权威一复制”双重可见

但它只解决“看起来别重复画”的问题，不解决“系统查询误把复制体算进模拟”的问题。  
因此 Host 上最危险的不是纯粹的 UI 重影，而是：

- 模拟仍在权威实体上跑
- 某些查询又把复制体算进集合
- 最终造成主机行为、客户端行为和 Host 本地视觉三者都可能出现差异

## 5. 关键接口与当前库存

### 5.1 关键接口

- `CoopNetConfig`
  - 联机模式与目标 IP
- `CoopNetState`
  - 联机生命周期状态、输入缓存、命令缓存
- `CoopInputState`
  - 网络上传输的按键/移动/瞄准快照
- `CoopCommandMessage`
  - 阶段型交互命令
- `CoopSessionState`
  - 当前 Coop 阶段、房间类型、房间状态、奖励/选门/RPS/商店/胜负信息
- `CoopPhase`
  - `None / Reward / DoorChoice / Rps / Shop / MatchOver`

### 5.2 未复制但被客户端 UI / 世界表现间接依赖的资源与组件

| 项目 | 谁在依赖 | 影响 |
| --- | --- | --- |
| `FloorLayout` | `ui::hud::update_room_text()`、`update_stage_progress()`、`map::tiles::refresh_room_decor()` | 客户端房间名、路线进度、房间装饰可能为空或不更新 |
| `CurrentRoom` | 同上 | 客户端无法知道当前房间编号/类型 |
| `RoomState` | `ui::hud::update_room_text()`、`update_hint_text()` | 客户端房间状态文案可能缺失或错误 |
| `FloorNumber` | `ui::hud::update_floor_text()`、`update_stage_progress()` | 客户端楼层信息可能为空 |
| `VisitedRooms` | 进度统计类 UI | 当前仓库里 minimap 更新未挂载，但后续一旦启用会立刻受影响 |
| `Transform` / `Sprite` | 复制实体显示 | 不是网络复制结果，而是本地补出来的显示层 |
| `EnemyStats` / `RewardModifiers` / 各类冷却、能量、攻击参数 | 如果客户端未来 UI 或特效直接读这些组件 | 单机能读到，联机客户端默认没有，容易出现“逻辑有但 UI 没数据” |
| `ActivePuzzle` / 谜题实体 | 谜题世界表现与完成状态 | 当前 Coop 下既不复制，也没有对应运行时 |

## 6. 高风险差异矩阵

| 优先级 | 风险 | 确认度 | 为什么会造成“单机成功、联机失败” | 最小验证动作 |
| --- | --- | --- | --- | --- |
| P0 | `127.0.0.1` / 手填 IP / UDP 3457 的局域网假设 | 代码已确认 | Host 本地 client 固定连 `127.0.0.1`，大厅文案也显示 `127.0.0.1`；远端机器若照着填必然失败。README 里本地脚本也已过时，容易误导排查。 | 在两台机器上复现时，明确记录 `CoopNetConfig.host_ip`；Host 用实际局域网 IP，确认防火墙已开放 UDP `3457`；在 `sync_client_connect_events()` / `sync_server_connect_events()` 打日志看是否真的握手成功。 |
| P0 | 单机奖励系统没有隔离出 Coop | 代码已确认 | `rewards::systems::enter_reward_selection()` 和 `offer_reward_in_reward_room()` 是全局 `Update` 系统，不只在 `InGame` 运行。Host 在 `CoopGame` 也有 `FloorLayout` / `CurrentRoom` / `RoomClearedEvent`，因此可能被单机奖励流抢状态，直接切到 `RewardSelect`。客户端没有这些资源，结果就是 Host / Client 状态分叉。 | 在 Host 上给 `enter_reward_selection()`、`offer_reward_in_reward_room()` 加日志，确认 `AppState::CoopGame` 时是否仍被执行；观察是否出现 Host 进入 `RewardSelect` 而客户端仍停在 `CoopGame`。 |
| P0 | Coop 生成了 Puzzle 房，但 Puzzle 运行时不在 `CoopGame` 中执行 | 代码已确认 | `room_entry_spawner()` 在 Coop 下仍会为 `RoomType::Puzzle` 调用 `spawn_puzzle_for_room()`；但 `PuzzlePlugin` 只在 `AppState::InGame` 运行。结果是 Host 会生成谜题房和谜题实体，却没有完成逻辑；客户端也拿不到谜题复制体。 | 在 Coop 谜题房打印 `ActivePuzzle`、`PuzzleEntity` 数量，并确认 `pressure_plate_system()` / `switch_order_system()` / `trap_system()` 在 `CoopGame` 下根本不跑。 |
| P0 | Host 进程中权威实体与复制实体并存，但部分玩法查询没有统一排除 `Replicated` | 代码已确认 | 这会把复制体误算进模拟。高风险点包括：`combat::projectiles::move_projectiles()`、`despawn_out_of_room_projectiles()`、`enemy::ai::update_enemy_ai()` 的敌人快照、`enemy::systems::enemy_attack_system()` 的支援怪候选与敌人位置快照、`enemy_death_system()` 的 `enemies_left`、`ui::hud::update_enemy_count_text()`。 | 在 Host 上分别打印 `With<Enemy>` 与 `With<Enemy>, Without<Replicated>` 数量，`With<Projectile>` 与 `Without<Replicated>` 数量；对比 Host 本地显示与远端客户端显示。 |
| P1 | 客户端缺少 `FloorLayout` / `CurrentRoom` / `RoomState` / `FloorNumber`，但 HUD 与房间装饰依赖这些本地资源 | 代码已确认 | Host 创建并持有这些资源；客户端主要只拿到 `CoopSessionState` 和实体组件。结果是客户端 HUD 的房间名、楼层、进度、提示文案可能为空或不一致，房间装饰也不会按房型刷新。 | 在客户端 `OnEnter(CoopGame)` 时检查这些资源是否存在；对照 `ui::hud::*` 与 `map::tiles::refresh_room_decor()` 的输入资源。 |
| P1 | 客户端进入 `CoopGame` 的条件只是“连接成功”，不是“会话和世界已准备好” | 代码已确认 | `auto_advance_lobby_state()` 对 Client 只检查 `net.connected`。如果连接一成立就切状态，客户端可能先进入 `CoopGame`，但复制的 `CoopSessionState`、本地 `LocalControlled` 玩家、门和敌人还没到，出现短暂空场或 UI 缺数据。 | 在 Client 的 `OnEnter(CoopGame)` 打日志，记录是否已存在 `CoopSessionState`、`LocalControlled` 玩家、复制门/敌人。 |
| P1 | 复制范围是手工白名单，未复制对象会天然缺失 | 代码已确认 | 当前只复制玩家/敌人/门/投射物/会话状态。谜题实体、商店亭、普通粒子、伤害数字、命中框、很多属性组件都不会出现在客户端。单机里这些都在同一世界，自然不会暴露问题。 | 逐项核对 `register_component()` 与客户端显示/功能依赖；尤其关注谜题、商店场景物和任何以后新增 UI。 |
| P2 | `CoopCommandMessage::SelectDoor` / `LeaveSession` 当前是死分支 | 代码已确认 | 协议看起来支持“命令式选门/离开会话”，但现网实现没有走这条链路。排查时如果只盯消息收发，会误判门逻辑没问题。 | 给 `flush_pending_client_commands()` 打日志，确认不会出现 `SelectDoor` / `LeaveSession`；同时在 `host_handle_door_interactions()` 看真实门选择来源。 |
| P2 | `CoopNetState.peer` 在 Coop 中不是实际连接配置源 | 代码已确认 | 它容易让人误以为改了 `peer` 就改了目标地址；实际上 Lightyear 用的是 `CoopNetConfig.host_ip`。 | 故意只改 `net.peer`、不改 `config.host_ip`，观察连接目标不会变化。 |
| P2 | 房间世界表现是“部分本地生成、部分网络复制”的混合体 | 代码已确认 | 通用地板/墙来自 `TilesPlugin`，任何端都能本地生成；门来自网络复制；房间装饰又依赖本地 `FloorLayout/CurrentRoom`。最终容易出现“基础场景有了，但房型表达和 HUD 对不上”。 | 在 Host/Client 同时截图同一房间，对比地板、门、房间标签、HUD 房间文案是否一致。 |

## 7. 当前最值得先看的代码位置

如果后续要正式 Debug，建议先看这些点：

1. `src/coop/net.rs`
   - `sync_coop_network_lifecycle()`
   - `sync_client_connect_events()`
   - `sync_server_connect_events()`
   - `auto_advance_lobby_state()`
2. `src/coop/runtime.rs`
   - `host_bootstrap_match()`
   - `host_tag_replicated_entities()`
   - `host_sync_network_views()`
   - `host_process_phase_commands()`
   - `host_handle_door_interactions()`
3. `src/coop/ui.rs`
   - `attach_replicated_visuals()`
   - `sync_replicated_visuals()`
   - `update_coop_overlay()`
   - `handle_coop_overlay_input()`
4. `src/gameplay/rewards/systems.rs`
   - 全局 `Update` 奖励系统是否误入 Coop
5. `src/gameplay/puzzle/mod.rs`
   - 只在 `InGame` 运行，是 Coop 谜题房缺口
6. `src/gameplay/combat/projectiles.rs`
   - 是否把复制投射物当成权威投射物继续移动/出界销毁
7. `src/gameplay/enemy/ai.rs` 与 `src/gameplay/enemy/systems.rs`
   - 是否把复制敌人纳入快照、支援怪 buff 候选和敌人计数
8. `src/ui/hud.rs`
   - 哪些 HUD 项依赖 Host-only 资源而不是复制态

## 8. 本地调试入口

README 里提到的：

- `local_debug/run_coop_local_debug.ps1`
- `local_debug/run_pvp_local_debug.ps1`
- `local_debug/cleanup_local_debug.ps1`

当前仓库中并不存在。  
现网真实入口是 `src/core/local_debug.rs` 里的 `LocalDebugPlugin`，通过环境变量驱动。

### 8.1 建议的单机双开方式

Host 窗口：

```powershell
$env:LOCAL_NET_DEBUG="1"
$env:LOCAL_NET_DEBUG_MODE="coop"
$env:LOCAL_NET_DEBUG_ROLE="host"
cargo run
```

Client 窗口：

```powershell
$env:LOCAL_NET_DEBUG="1"
$env:LOCAL_NET_DEBUG_MODE="coop"
$env:LOCAL_NET_DEBUG_ROLE="client"
$env:LOCAL_NET_DEBUG_HOST="127.0.0.1"
cargo run
```

说明：

- Host 默认窗口位置在左上，Client 在右上。
- 本地调试会自动跳到 `CoopLobby`。
- 如果要排“真局域网问题”，不要只做本机双开；本机双开只能验证逻辑，不能验证防火墙、网卡、真实 LAN 地址。

## 9. Debug 清单

### 9.1 复现环境矩阵

| 场景 | 目标 | 最先确认什么 |
| --- | --- | --- |
| 单机双开 | 验证状态流、输入流、复制流和 Host/Client 视觉差异 | `local_connected` / `remote_connected`、`CoopSessionState` 是否到达、Host 是否出现复制体污染 |
| 同一局域网两台机器 | 验证真实网络连通性 | Host 实际局域网 IP、UDP `3457`、两端防火墙、客户端输入的 `host_ip` |
| 错误 IP / 防火墙场景 | 验证失败症状是否可区分 | Client 只会停在 `CoopLobby` 且 `connected=false`；Host 只会 `local_connected=true, remote_connected=false` |

### 9.2 按症状排查

**症状 A：根本连不上**

- 看 `CoopNetConfig.host_ip` 是否真的是 Host 的局域网 IP，不是 `127.0.0.1`
- 看 `sync_coop_network_lifecycle()` 是否真的启动了 server/client
- 看 Host 是否只 `local_connected=true`，但 `remote_connected=false`
- 看系统防火墙是否放行 UDP `3457`

**症状 B：能进房，但客户端世界明显不一样**

- 先对比 Host / Client 是否都拿到了复制 `CoopSessionState`
- 再对比 Host 上 `Enemy` / `Projectile` 的总数与 `Without<Replicated>` 数量
- 检查 `attach_replicated_visuals()` 是否给复制体补齐了显示层
- 检查是否碰到了 Puzzle 房，或奖励流误切到了 `RewardSelect`

**症状 C：输入到了，但效果和主机不一致**

- 看 Host 的 `latest_inputs` 是否在持续刷新
- 看 `host_buffer_player_inputs()` 里 `P2` 是否拿到了远端输入
- 看门交互是否误以为走 `SelectDoor`，实际上它不走消息，而是走 Host 侧站位 + `interact_pressed`
- 看 Host 是否因为复制体污染导致远端玩家附近的敌人/投射物逻辑异常

**症状 D：主机和客户端看到的敌人/投射物数量不同**

- 首查 `projectiles::move_projectiles()` / `despawn_out_of_room_projectiles()`
- 首查 `enemy::ai::update_enemy_ai()` 的快照集合
- 首查 `enemy_attack_system()` 的 `enemy_positions`
- 首查 `enemy_death_system()` 的 `enemies_left`
- 首查 `ui::hud::update_enemy_count_text()`

**症状 E：房间信息 / 楼层信息 / 提示文案不同**

- 看客户端是否拥有 `FloorLayout`
- 看客户端是否拥有 `CurrentRoom`
- 看客户端是否拥有 `RoomState`
- 看客户端是否拥有 `FloorNumber`
- 如果这些资源都没有，不要先怀疑 HUD 本身，先承认它当前依赖的是 Host-only 资源

### 9.3 推荐排查顺序

1. 先排网络连通：`host_ip`、端口、防火墙、连接事件。
2. 再排状态边界：Client 是不是过早进入 `CoopGame`。
3. 再排 Host 污染：权威实体和复制实体是否被同一套系统混算。
4. 再排单机残留：奖励系统是否误入 Coop，Puzzle 房是否根本没运行时。
5. 最后再看纯表现问题：HUD、房间装饰、门颜色、远端血条、粒子/特效。

## 10. 当前文档可直接支撑的后续动作

- 给 `src/coop/net.rs` 和 `src/coop/runtime.rs` 加临时日志，先把连接、状态切换、会话生成时序跑清楚。
- 给 Host 上的 `Enemy` / `Projectile` 统计增加“总数 vs `Without<Replicated>`”对照。
- 明确决定 Puzzle 房在 Coop 中是：
  - 暂时禁用生成；
  - 还是补齐 `CoopGame` 运行时与复制方案。
- 明确决定奖励系统是：
  - 完全由 `CoopRuntimePlugin` 接管；
  - 还是继续让单机 `RewardSelect` 参与，但那需要额外的联机状态设计。

在这些问题没有收敛之前，“单机成功但联机失败”会持续出现，而且大概率不是单点 bug，而是状态边界、复制白名单和单机残留系统共同作用的结果。
