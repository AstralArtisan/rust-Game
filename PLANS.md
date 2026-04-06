# Sub-Phase 5c: TideHunter 数值调整 + 机制增强

## Context

Floor 3 Boss TideHunter 威胁偏低。需要缩短 stalk 时间（更频繁攻击）、提升暗影伤害、增加 ShadowDash 接触伤害、P2+ Telegraph 阶段加入目标预判。

## Affected files

| 文件 | 操作 |
|------|------|
| `src/gameplay/enemy/boss.rs` | 修改：数值调整 + 接触伤害 + 目标预判 |

## 详细改动

### 1. `apply_tide_hunter_phase_params` 数值调整（约第 615-636 行）

修改 `stalk_duration_s` 和 `shadow_duration_s`：

| Boss Phase | 旧 stalk_duration_s | 新 stalk_duration_s | 旧 shadow_duration_s | 新 shadow_duration_s |
|------------|---------------------|---------------------|----------------------|----------------------|
| P1 | 1.8 | 1.2 | 2.5 | 2.5（不变） |
| P2 | 1.4 | 0.8 | 3.5 | 3.5（不变） |
| P3 | 1.0 | 0.5 | 4.5 | 6.0 |

具体代码：在 `apply_tide_hunter_phase_params` 函数中：
- P1 分支：`state.stalk_duration_s = 1.8` → `state.stalk_duration_s = 1.2`
- P2 分支：`state.stalk_duration_s = 1.4` → `state.stalk_duration_s = 0.8`
- P3 分支（`_ =>`）：`state.stalk_duration_s = 1.0` → `state.stalk_duration_s = 0.5`，`state.shadow_duration_s = 4.5` → `state.shadow_duration_s = 6.0`

### 2. Shadow damage multiplier 提升（约第 342 行）

在 `TideHunterPhase::Telegraph` 分支中，`spawn_shadow_trail_line` 调用的 damage 参数：

```rust
// 旧：
stats.attack_damage * 0.6,
// 新：
stats.attack_damage * 1.0,
```

即将 `0.6` 改为 `1.0`。

### 3. ShadowDash 接触伤害（新系统）

新增系统 `tide_hunter_contact_damage_system`，注册到 `BossPlugin` 的 Update schedule，run_if 同其他 boss 系统。

逻辑：
- 查询所有 `TideHunterState` 且 `phase == ShadowDash` 的 boss 实体的位置
- 查询所有 `Player`（非 Replicated）的位置和 `Health`
- 如果 boss 与 player 距离 < 30.0，对 player 造成 `stats.attack_damage * 0.5` 的伤害
- 使用一个简单的冷却机制避免同一次 dash 多次命中：在 `TideHunterState` 中新增 `contact_hit_cooldown: Timer`（0.3s），每帧 tick，只有 finished 时才造成伤害并 reset

需要修改 `src/gameplay/enemy/components.rs` 的 `TideHunterState` 结构体，新增字段：
```rust
pub contact_hit_cooldown: Timer,
```

在 boss.rs 中 TideHunterState 的初始化位置（搜索 `TideHunterState {`），给 `contact_hit_cooldown` 赋初始值：
```rust
contact_hit_cooldown: Timer::from_seconds(0.0, TimerMode::Once),
```
（初始 finished=true，这样第一次接触立即生效）

系统签名：
```rust
fn tide_hunter_contact_damage_system(
    time: Res<Time>,
    mut tide_q: Query<(&GlobalTransform, &EnemyStats, &mut TideHunterState)>,
    mut player_q: Query<(&GlobalTransform, &mut Health), (With<Player>, Without<Replicated>)>,
    mut damage_events: EventWriter<DamageAppliedEvent>,
) {
    for (boss_tf, stats, mut state) in &mut tide_q {
        if state.phase != TideHunterPhase::ShadowDash {
            continue;
        }
        state.contact_hit_cooldown.tick(time.delta());
        if !state.contact_hit_cooldown.finished() {
            continue;
        }
        let boss_pos = boss_tf.translation().truncate();
        let contact_damage = stats.attack_damage * 0.5;
        for (player_tf, mut hp) in &mut player_q {
            let dist = boss_pos.distance(player_tf.translation().truncate());
            if dist < 30.0 {
                hp.current = (hp.current - contact_damage).max(0.0);
                state.contact_hit_cooldown = Timer::from_seconds(0.3, TimerMode::Once);
                damage_events.send(DamageAppliedEvent {
                    target: player_tf.translation().truncate().extend(0.0),
                    damage: contact_damage,
                    is_crit: false,
                });
                break; // one hit per tick
            }
        }
    }
}
```

注意：`DamageAppliedEvent` 和 `Health` 需要从现有 crate 导入。搜索现有 import 找到正确路径。

### 4. P2+ Telegraph 目标预判（约第 329-330 行）

在 `TideHunterPhase::Telegraph` 分支中，`compute_tide_hunter_dash_target` 的调用目前传入 `player_pos`。

修改：系统需要访问玩家的 `Velocity` 组件。将 player query 改为也获取 `Option<&Velocity>`（`Velocity` 来自 `crate::gameplay::player::components::Velocity`）。

在 Telegraph 分支中，当 `boss_phase.0 >= 2` 时，用预判位置替代原始 player_pos：
```rust
let predicted_pos = if boss_phase.0 >= 2 {
    if let Some(vel) = player_vel {
        player_pos + vel.0 * 0.3
    } else {
        player_pos
    }
} else {
    player_pos
};
state.dash_target = compute_tide_hunter_dash_target(state.dash_start, predicted_pos, &state);
```

这需要修改 `tide_hunter_system` 的 player_q 签名，从：
```rust
player_q: Query<(&GlobalTransform, Option<&GhostState>), (With<Player>, Without<Replicated>)>,
```
改为：
```rust
player_q: Query<(&GlobalTransform, Option<&GhostState>, Option<&Velocity>), (With<Player>, Without<Replicated>)>,
```

并在收集 player_positions 时同时收集 velocity：
```rust
let player_data: Vec<(Vec2, Option<Vec2>)> = player_q
    .iter()
    .filter_map(|(tf, ghost, vel)| {
        (!matches!(ghost, Some(GhostState::Ghost))).then_some((
            tf.translation().truncate(),
            vel.map(|v| v.0),
        ))
    })
    .collect();
```

然后在循环中找最近玩家时也获取其 velocity，用于 Telegraph 预判。

## Affected files（更新）

| 文件 | 操作 |
|------|------|
| `src/gameplay/enemy/boss.rs` | 修改：数值 + shadow_damage_mult + 接触伤害系统 + 目标预判 |
| `src/gameplay/enemy/components.rs` | 修改：TideHunterState 新增 contact_hit_cooldown 字段 |

## 验证

```bash
cargo check --quiet && cargo test --quiet
```
