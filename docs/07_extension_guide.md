# 扩展与维护指南

- 适用版本：当前工作树（branch `claude-playground`）
- 最后校验：2026-04-08；结合当前模块结构、配置入口与联机实现整理
- 关联源码：`src/gameplay/`、`src/data/`、`src/coop/`、`src/pvp/`、`src/ui/`、`assets/configs/`
- 实验性内容：包含。联机相关扩展项需要按原型架构审慎推进

## 1. 扩展前的统一检查
无论新增什么内容，先回答五个问题：

1. 它属于单机专属、Coop 共享、还是 PVP 专属？
2. 它的平衡参数是否应该配置化？
3. 它是否需要新的状态、事件或网络消息？
4. 它是否会影响 HUD、奖励、商店、存档或成就？
5. 它是否应该先下沉到 `session_core`，而不是直接写死在某个 UI 或运行时文件里？

## 2. 新增敌人
### 2.1 典型触点
- `src/gameplay/enemy/components.rs`
- `src/data/definitions.rs`
- `assets/configs/enemies.ron`
- `src/gameplay/enemy/ai.rs`
- `src/gameplay/enemy/systems.rs`
- `src/gameplay/enemy/spawner.rs`

### 2.2 推荐步骤
1. 在 `EnemyType` 中增加新枚举。
2. 为它补充配置结构消费路径。
3. 在 `enemies.ron` 中给出数值模板。
4. 在 `ai.rs` 中补行为分支。
5. 在 `spawner.rs` 或刷怪逻辑里决定它的生成条件。
6. 检查是否需要新的投射物、特效或 Boss 互动。
7. 同步更新文档和测试说明。

### 2.3 常见遗漏
- 只加了配置，没加生成入口
- 只加了 AI，没加受击/死亡特化表现
- 忘了考虑 Coop 中是否需要复制展示

## 3. 新增奖励
### 3.1 典型触点
- `src/gameplay/rewards/data.rs`
- `src/gameplay/rewards/apply.rs`
- `src/gameplay/session_core/mod.rs`
- `assets/configs/rewards.ron`
- `src/ui/reward_select.rs`
- 可能还包括 `src/ui/hud.rs`

### 3.2 推荐步骤
1. 在 `RewardType` 中新增枚举。
2. 在 `rewards.ron` 中补充标题、描述和值。
3. 在 `apply.rs` 中实现对玩家属性或行为的影响。
4. 在 `session_core` 的奖励池与草案生成逻辑中接入它。
5. 检查单机奖励页和 Coop 奖励 UI 是否都能正确展示。
6. 若奖励影响玩家派生能力，更新 `RewardModifiers` 或相关组件。
7. 为规则层补测试。

### 3.3 维护原则
- 不要只改文本，不改实际效果
- 不要让单机和 Coop 使用不同的奖励语义
- 奖励曲线优先统一在 `session_core`

## 4. 新增房间类型
### 4.1 典型触点
- `src/gameplay/map/room.rs`
- `src/gameplay/map/generator.rs`
- `src/gameplay/enemy/systems.rs`
- `src/gameplay/session_core/mod.rs`
- `src/ui/hud.rs`
- 可能还包括 `src/gameplay/shop/` 或 `src/gameplay/puzzle/`

### 4.2 推荐步骤
1. 在 `RoomType` 中增加枚举。
2. 在地图生成器里定义生成策略。
3. 在房间进入逻辑中定义是否锁门、刷怪、生成事件对象。
4. 在 HUD/提示文本中补充房间标签。
5. 在共享规则层中决定清房后是否给奖励、是否自动开商店、是否直通下一层。
6. 检查单机与 Coop 是否都需要支持。

### 4.3 高风险点
- 房间类型改动会同时影响地图、敌人、商店、奖励、HUD 和联机阶段
- 如果只在单机支持，文档里必须明确标注

## 5. 新增 Boss 或 Boss 阶段
### 5.1 典型触点
- `src/gameplay/enemy/components.rs`
- `src/gameplay/enemy/boss.rs`
- `assets/configs/boss.ron`
- `src/ui/hud.rs`
- `src/core/events.rs`

### 5.2 推荐步骤
1. 决定是新增 `BossArchetype` 还是扩展现有 Boss 阶段。
2. 在 `boss.ron` 中补数值。
3. 在 `boss.rs` 中接入阶段阈值、攻击模式和阶段切换。
4. 若需要新提示，使用 `BossPhaseChangeEvent` 和 HUD/Boss 血条。
5. 检查单机与 Coop Host 是否都能复用。

## 6. 新增 UI 页面
### 6.1 典型触点
- `src/ui/mod.rs`
- 新页面文件，例如 `src/ui/<page>.rs`
- `src/states.rs`
- 相关业务模块的状态切换逻辑

### 6.2 推荐步骤
1. 先决定它是否需要独立 `AppState`。
2. 在 `UiPlugin` 中注册 `OnEnter` / `Update` / `OnExit`。
3. 页面必须配套 cleanup。
4. 如果它依赖 gameplay 数据，优先从资源/事件读取，不要直接写玩法状态。
5. 如果它是 Coop 专属页面，优先考虑放在 `coop/ui.rs` 而不是通用 `ui/`。

## 7. 新增配置项
### 7.1 典型触点
- `src/data/definitions.rs`
- `src/data/loaders.rs`
- `src/data/registry.rs`
- 对应 `assets/configs/*.ron`

### 7.2 推荐步骤
1. 先在 `definitions.rs` 增字段。
2. 更新 `.ron` 文件。
3. 更新 `default_registry()` 默认值。
4. 在消费它的 gameplay 模块中接线。
5. 如果会影响平衡，更新文档和校验说明。

## 8. 新增 Coop 阶段或联机交互
### 8.1 典型触点
- `src/coop/components.rs`
- `src/coop/net.rs`
- `src/coop/runtime.rs`
- `src/coop/ui.rs`
- 必要时 `src/gameplay/session_core/mod.rs`

### 8.2 推荐步骤
1. 判断它是纯状态同步，还是需要命令消息。
2. 如果需要阶段，先扩 `CoopPhase`。
3. 如果需要同步数据，扩 `CoopSessionState` 的对应子状态。
4. 如果需要客户端提交操作，扩 `CoopCommandMessage`。
5. 在 Host 运行时处理真实逻辑。
6. 在 UI 层只做输入采集和状态展示。
7. 补充本机双开验证步骤。

### 8.3 原则
- 不要把玩法决策直接塞进 `coop/ui.rs`
- 不要跳过 `CoopSessionState` 私自做“本地猜测状态”
- 如能抽象到 `session_core`，优先抽象

## 9. 新增 PVP 消息或对局能力
### 9.1 典型触点
- `src/pvp/net.rs`
- `src/pvp/systems.rs`
- `src/pvp/ui.rs`

### 9.2 推荐步骤
1. 先判断它属于输入、状态快照还是即时事件。
2. 扩展 `PvpMsg` 及对应结构体。
3. 同时修改 Host 和 Client 的收发分支。
4. 检查本地预测是否仍成立。
5. 确认结果页和 HUD 是否需要同步更新。

## 10. 扩展后最低验证清单
### 10.1 通用
1. `cargo check`
2. `cargo test`
3. 手动跑一轮 `cargo run`

### 10.2 如果改了单机玩法
1. 主菜单进入单机
2. 验证房间推进
3. 验证奖励/商店/结算
4. 验证 `F5` / `F9`

### 10.3 如果改了 Coop
1. 本机双开 Host/Client
2. 验证连接建立
3. 验证进入 `CoopGame`
4. 验证相关交互阶段和断开恢复

### 10.4 如果改了 PVP
1. 本机双开
2. 验证 `Hello/Welcome`
3. 验证状态同步和结果页

## 11. 文档同步要求
以下改动完成后，必须同步文档：

- 新增 `AppState` 或 `RoomType`
- 新增配置文件或新增配置字段
- 新增奖励、敌人、Boss、房间类型
- 新增联机阶段、网络消息或联调入口
- 更改多人地址输入规则或端口

最低同步位置：

- `README.md`
- `docs/03_module_design.md`
- `docs/04_api_and_data_model.md`
- 如涉及联机，再更新 `docs/06_multiplayer_and_risks.md`

## 12. 最后建议
- 新增功能时，优先沿着现有模块边界扩展，不要把临时逻辑堆进大文件
- 复杂规则优先下沉到 `session_core`
- 任何联机改动都要先确认所属体系是 Coop 还是 PVP
- 任何“只是 UI 改动”的说法都值得警惕，因为本项目 UI 往往绑定着状态机和流程语义
