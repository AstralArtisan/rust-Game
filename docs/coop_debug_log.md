# Coop 调试日志

最后更新：2026-04-02


## 本轮目标

- 收紧 authority / replicated 的玩法边界
- 固化 Client 进入 `CoopGame` 的最小就绪条件
- 为 Coop 房间类型归一化与多人 readiness 补最小测试

## 已确认的现状

- HUD fallback、Lobby gating、Reward/Shop/RPS overlay、复制体视觉同步在当前分支已经有基础实现
- 旧计划里有一部分“待实现项”已经不是事实，不能再按旧方案重复开发
- Host 上 authority 实体和 replicated 展示体并存是设计选择，问题在于查询边界而不是实体数量本身

## 本轮代码改动

### 1. Combat authority 边界显式化

- `src/gameplay/combat/projectiles.rs`
  - `despawn_expired_projectiles()` 只处理 `Without<Replicated>`
- `src/gameplay/combat/hitbox.rs`
  - 近战反弹、命中判定、hitbox 生命周期、rupture tick 全部显式排除 `Replicated`
- `src/gameplay/combat/damage.rs`
  - 伤害结算与击退衰减显式排除 `Replicated`
- `src/gameplay/enemy/systems.rs`
  - 房间进入清理与清房后敌方攻击清理显式排除 `Replicated`

### 2. Lobby readiness 规则补测试

- `src/coop/net.rs`
  - 抽出本地玩家槽位与 readiness 判定辅助函数
  - 增加“条件不足不进场”和“session/player/door 就绪后进场”的测试

### 3. Coop 房间归一化补测试

- `src/coop/runtime.rs`
  - 增加 `Puzzle -> Normal` 归一化测试

## 新增测试

- `client_lobby_waits_for_full_replicated_world_before_entering_game`
- `client_lobby_enters_game_once_session_player_and_door_are_ready`
- `despawn_expired_projectiles_keeps_replicated_visuals_outside_authority_loop`
- `detect_hitbox_hurtbox_overlap_ignores_replicated_hitboxes`
- `coop_layout_normalization_rewrites_puzzle_rooms_to_normal`

## 验证结果

- `cargo test --quiet` 通过
- `cargo check --quiet` 通过
- `cargo clippy --quiet --tests -- -D warnings` 未执行
  - 原因：当前工具链未安装 Clippy

## 未完成项

- 本轮没有做图形化双开或局域网双机验收
- 远端近战刀光 / dash 残影链路在代码上已完成基线校验，但仍缺少真实运行时截图或录像级验证
## 本轮目标

- 阶段 1：修复 RPS 永久等待问题，并清理 `src/coop/ui.rs` 中残留的英文联机文案。

## 本轮代码改动

- `src/coop/components.rs`
  - 为 `CoopRpsState` 新增 `input_timeout_s`，用于记录双方出拳等待倒计时。
- `src/coop/runtime.rs`
  - 为 RPS 等待阶段加入 12 秒超时兜底。
  - 超时后为未出拳玩家自动补全出拳，并在平局重开时重置倒计时。
- `src/coop/ui.rs`
  - 为 RPS 弹窗加入剩余倒计时提示。
  - 将联机菜单、大厅、暂停弹窗、商店刷新项等残留英文统一改为中文。

## 新增测试（如有）

- `default_rps_state_starts_with_timeout_budget`
- `reset_rps_input_round_clears_choices_and_restores_timeout`
- `fill_missing_rps_choices_assigns_missing_inputs`

## 验证结果

- `cargo test --quiet` 通过（32/32）
- `cargo check --quiet` 通过
- 双机联调尚未执行；RPS 超时、联机 UI 中文化目前仅完成代码级与单元测试验证

## 本轮目标

- 阶段 2：修复联机近战刀光起点使用插值坐标导致的偏移。

## 本轮代码改动

- `src/coop/ui.rs`
  - 远端玩家触发近战刀光时，优先使用 `CoopNetPosition` 的权威坐标计算刀光起点；仅在复制坐标缺失时回退到本地 `Transform`。

## 新增测试（如有）

- 无

## 验证结果

- `cargo test --quiet` 通过（32/32）
- `cargo check --quiet` 通过
- 双机联调尚未执行；刀光位置修复目前完成代码级验证

## 本轮目标

- 阶段 3：修复远端玩家冲刺残影在网络抖动下的单帧闪烁。

## 本轮代码改动

- `src/coop/ui.rs`
  - 为 `ReplicatedPlayerVisualState` 增加 dash 结束 grace 窗口状态。
  - 将残影激活判断改为“有效 dash 状态”，仅在连续超过 90ms 的非激活后才确认 dash 结束并重置残影计时器。

## 新增测试（如有）

- 无

## 验证结果

- `cargo test --quiet` 通过（32/32）
- `cargo check --quiet` 通过
- 双机联调尚未执行；残影平滑修复目前完成代码级验证
