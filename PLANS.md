# 单机模式改进 — 第二阶段：Boss差异化

## Context

4个Boss目前体型相同（64px）、移动方式相同（直线追玩家），攻击模式都是弹幕变体（扇形/环形/螺旋/弹幕墙），战斗身份缺乏辨识度。本次改动为每个Boss建立独特战斗身份和核心机制，参考 Hades / Hollow Knight / Isaac 同类设计：

| Boss | 战斗身份 | 核心机制 |
|------|---------|---------|
| Floor1Guardian | 沉重坦克 | 防御朝向：正面减伤 60%，需绕背打弱点 |
| MirrorWarden | 幻影术师 | 传送时留下幻象，幻象发弹但被命中不闪白 |
| TideHunter | 人形猎手 | 小体型极速，Phase1 零弹幕，可格挡反制冲刺 |
| CubeCore | 方块要塞 | 4 个浮动子核心，全存活时主体完全免疫 |

---

## 影响文件

| 文件 | 改动类型 |
|------|---------|
| `assets/configs/boss.ron` | 修改：HP/速度数值 |
| `src/gameplay/enemy/components.rs` | 修改：新增 6 个组件/枚举 |
| `src/gameplay/enemy/systems.rs` | 修改：Boss 体型、附加新组件、CubeCore 召唤子核心 |
| `src/gameplay/enemy/boss.rs` | 修改：MirrorWarden 幻象生成、TideHunter Phase1 逻辑，新增系统 |
| `src/gameplay/enemy/ai.rs` | 修改：Boss 移动个性化 |
| `src/gameplay/combat/damage.rs` | 修改：伤害管线加入 Boss 机制过滤 |
| `src/ui/tutorial.rs` | 修改：新增 Boss 机制专用提示触发点 |

---

## 步骤一：数值调整

### `assets/configs/boss.ron`
```ron
(
  floor_1: (
    max_hp: 300.0,
    move_speed: 95.0,        // 更慢，更重
    contact_damage: 14.0,
    phase_thresholds: [0.60, 0.30],
    projectile_speed: 430.0,
  ),
  floor_2: (
    max_hp: 340.0,
    move_speed: 130.0,       // 稍快，但靠传送
    contact_damage: 15.0,
    phase_thresholds: [0.68, 0.34],
    projectile_speed: 470.0,
  ),
  floor_3: (
    max_hp: 360.0,
    move_speed: 175.0,       // 极快
    contact_damage: 22.0,    // 高接触伤害替代弹幕
    phase_thresholds: [0.70, 0.35],
    projectile_speed: 505.0, // Phase2+ 才用到
  ),
  floor_4: (
    max_hp: 800.0,
    move_speed: 82.0,        // 极慢但压迫感强
    contact_damage: 19.0,
    phase_thresholds: [0.72, 0.38],
    projectile_speed: 540.0,
  ),
)
```

### `src/gameplay/enemy/systems.rs` — `spawn_boss` 体型

将原先统一的 64/68px 改为按 archetype 区分：

```rust
let (sprite_size, hurtbox_size) = match archetype {
    BossArchetype::Floor1Guardian => (72.0_f32, 68.0_f32),
    BossArchetype::MirrorWarden   => (60.0, 56.0),
    BossArchetype::TideHunter     => (32.0, 30.0),  // 人形猎手，极小
    BossArchetype::CubeCore       => (84.0, 80.0),
};
```

---

## 步骤二：新增组件 (`src/gameplay/enemy/components.rs`)

```rust
// ── Floor1Guardian ───────────────────────────────────────────
/// Boss 面朝方向。正面弧（dot(hit_dir, facing) > 0.4）受到伤害 ×0.4。
#[derive(Component, Debug, Clone, Copy)]
pub struct BossDirectionalDefense {
    pub facing: Vec2,
}

// ── MirrorWarden ─────────────────────────────────────────────
/// 幻象标记。命中不触发闪白，伤害归零。
#[derive(Component, Debug, Clone)]
pub struct BossDecoy {
    pub lifetime: Timer,
}

// ── TideHunter ───────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TideHunterPhase {
    Stalk,
    WindupTelegraph, // 橙色预警，parry 窗口开启
    Lunge,           // 高速冲刺，伤害 hitbox 存活
    Cooldown,        // 收招
    Stunned,         // 被格挡后的硬直
}

#[derive(Component, Debug, Clone)]
pub struct TideHunterState {
    pub phase: TideHunterPhase,
    pub timer: Timer,
    pub lunge_dir: Vec2,
    pub parry_window_active: bool,
}

// ── CubeCore ─────────────────────────────────────────────────
/// 子核心，附加在 CubeCore 周围的卫星小方块。
#[derive(Component, Debug, Clone)]
pub struct BossSubCore {
    pub boss_entity: Entity,
    pub orbit_angle: f32,  // 当前轨道角度（弧度）
    pub orbit_speed: f32,  // 转速（弧度/秒）
}

/// 主体护盾状态。cores_alive > 0 时主体完全免疫。
#[derive(Component, Debug, Clone, Copy)]
pub struct BossCoreShield {
    pub cores_alive: u8,
}
```

---

## 步骤三：Boss 移动个性化 (`src/gameplay/enemy/ai.rs`)

在 `update_enemy_ai` 的 `EnemyType::Boss` 分支**末尾**（速度已被设置后），新增一个独立系统 `boss_movement_override`，覆盖有 `BossArchetype` 的 Boss 速度。放在 `EnemySystemsPlugin` 中，在 `update_enemy_ai` 之后运行。

```rust
pub fn boss_movement_override(
    time: Res<Time>,
    player_q: Query<&GlobalTransform, (With<Player>, Without<Replicated>)>,
    mut q: Query<(
        &BossArchetype,
        &EnemyStats,
        &Transform,
        &mut EnemyVelocity,
        Option<&TideHunterState>,
    )>,
) {
    // 取最近玩家位置
    let Some(player_pos) = player_q.iter().map(|t| t.translation().truncate()).next() else { return };

    for (archetype, stats, tf, mut vel, tide_state) in &mut q {
        let pos = tf.translation.truncate();
        let dir = (player_pos - pos).normalize_or_zero();
        let speed = stats.move_speed;

        match archetype {
            BossArchetype::TideHunter => {
                // 由 TideHunterState 状态机控制速度
                if let Some(state) = tide_state {
                    vel.0 = match state.phase {
                        TideHunterPhase::Lunge => state.lunge_dir * speed * 5.0,
                        TideHunterPhase::Stunned | TideHunterPhase::WindupTelegraph => Vec2::ZERO,
                        // Stalk / Cooldown：快速绕圈靠近
                        _ => {
                            let orbit = Vec2::new(-dir.y, dir.x) * 1.6;
                            (dir * 0.6 + orbit).normalize_or_zero() * speed
                        }
                    };
                }
            }
            BossArchetype::CubeCore => {
                // 极慢，恒定向玩家施压
                vel.0 = dir * speed * 0.85;
            }
            _ => {} // Guardian / MirrorWarden 保持 update_enemy_ai 的默认行为
        }
    }
}
```

---

## 步骤四：Floor1Guardian — 防御朝向

### 新系统 `boss_guardian_facing_system` (boss.rs)

```rust
pub fn boss_guardian_facing_system(
    player_q: Query<&GlobalTransform, (With<Player>, Without<Replicated>)>,
    mut q: Query<(&Transform, &mut BossDirectionalDefense), With<BossArchetype>>,
) {
    let Some(player_pos) = player_q.iter().map(|t| t.translation().truncate()).next() else { return };
    for (tf, mut defense) in &mut q {
        let dir = (player_pos - tf.translation.truncate()).normalize_or_zero();
        // 缓慢转向
        defense.facing = defense.facing.lerp(dir, 0.06).normalize_or_zero();
    }
}
```

### `apply_damage_events` (damage.rs) — 防御过滤

在现有伤害应用逻辑中，判断 target 是否有 `BossDirectionalDefense`：

```rust
// 新增 Query 参数
directional_def_q: Query<&BossDirectionalDefense>,

// 在循环内，无敌帧检查之后：
if let Ok(defense) = directional_def_q.get(entity) {
    // ev.knockback 方向即打击来向（取反为朝向 Boss 的方向）
    let hit_from = -ev.knockback.normalize_or_zero();
    if hit_from.dot(defense.facing) > 0.4 {
        // 正面命中，减伤 60%
        amount *= 0.4;
    }
}
```

> 注：`ev.knockback` 为击退向量，方向从 Boss 指向外，取反即玩家打击方向。若 knockback 为零，跳过减伤判断。

### `spawn_boss` — 附加组件 (systems.rs)

```rust
if archetype == BossArchetype::Floor1Guardian {
    commands.entity(id).insert(BossDirectionalDefense { facing: Vec2::NEG_X });
}
```

---

## 步骤五：MirrorWarden — 幻象残像

### 修改 `run_floor_2_pattern` (boss.rs)

Phase 2 和 Phase 3 传送（`boss_tf.translation = anchor`）之前，在旧位置 `boss_pos` 生成幻象：

```rust
// 在更新 boss_tf.translation 之前：
spawn_mirror_decoy(commands, assets, boss_pos, stats, dir, phase);
```

### 新函数 `spawn_mirror_decoy`

```rust
fn spawn_mirror_decoy(
    commands: &mut Commands,
    assets: &GameAssets,
    pos: Vec2,
    stats: &EnemyStats,
    dir: Vec2,
    phase: u8,
) {
    let decoy_id = commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation(pos.extend(44.0)), // z 略低于 Boss
            sprite: Sprite {
                color: Color::srgba(0.58, 0.82, 1.0, 0.45), // MirrorWarden 颜色，50% 透明
                custom_size: Some(Vec2::splat(60.0)),
                ..default()
            },
            ..default()
        },
        BossDecoy { lifetime: Timer::from_seconds(2.8, TimerMode::Once) },
        InGameEntity,
        Name::new("MirrorDecoy"),
    )).id();
    // 幻象立即发射一波弹幕
    spawn_cross(commands, assets, pos, stats.projectile_speed * 0.75, stats.attack_damage * 0.35);
    if phase >= 3 {
        // Phase3 幻象多发一波扇形
        spawn_fan(commands, assets, pos + dir * 16.0, dir, stats.projectile_speed * 0.9, stats.attack_damage * 0.3, &[-0.22, 0.0, 0.22]);
    }
}
```

### 新系统 `boss_decoy_system`

```rust
pub fn boss_decoy_system(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut BossDecoy)>,
) {
    for (entity, mut decoy) in &mut q {
        decoy.lifetime.tick(time.delta());
        if decoy.lifetime.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}
```

### `apply_damage_events` — 幻象免伤 (damage.rs)

```rust
decoy_q: Query<(), With<BossDecoy>>,

// 在伤害循环最开始：
if decoy_q.contains(entity) {
    continue; // 幻象不受伤，不触发 Flash
}
```

---

## 步骤六：TideHunter — 人形猎手

### Phase1 零弹幕 (`boss.rs` → `run_floor_3_pattern`)

Phase 1 分支改为：**只由 TideHunterState 状态机驱动近战**，`run_floor_3_pattern` 的 Phase 1 分支不再生成弹幕，仅重置攻击计时器（时间设长，给状态机让路）：

```rust
1 => {
    *timer = Timer::from_seconds(2.0, TimerMode::Once); // 占位，实际攻击由状态机控制
    timer.reset();
}
```

Phase 2 和 3 保持现有弹幕逻辑，减少一层 `sidestep`，保留冲刺/召唤。

### 新系统 `tide_hunter_state_machine` (boss.rs)

```rust
pub fn tide_hunter_state_machine(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<GameAssets>,
    player_q: Query<&GlobalTransform, (With<Player>, Without<Replicated>)>,
    mut q: Query<(Entity, &Transform, &EnemyStats, &mut TideHunterState), With<BossArchetype>>,
) {
    let Some(player_pos) = player_q.iter().map(|t| t.translation().truncate()).next() else { return };

    for (boss_entity, tf, stats, mut state) in &mut q {
        let pos = tf.translation.truncate();
        let dist = pos.distance(player_pos);
        state.timer.tick(time.delta());

        match state.phase {
            TideHunterPhase::Stalk => {
                // 接近到攻击距离时进入预警
                if dist < 120.0 {
                    state.phase = TideHunterPhase::WindupTelegraph;
                    state.timer = Timer::from_seconds(0.38, TimerMode::Once);
                    state.timer.reset();
                    state.parry_window_active = true;
                    // 橙色闪白提示
                    // （需通过 commands 访问 Flash，此处可改为 Event 触发）
                }
            }
            TideHunterPhase::WindupTelegraph => {
                if state.timer.finished() {
                    state.phase = TideHunterPhase::Lunge;
                    state.timer = Timer::from_seconds(0.22, TimerMode::Once);
                    state.timer.reset();
                    state.lunge_dir = (player_pos - pos).normalize_or_zero();
                    state.parry_window_active = false;
                    // 生成近战 Hitbox
                    spawn_melee_hitbox(&mut commands, &assets, boss_entity, pos, state.lunge_dir, stats.attack_damage);
                }
            }
            TideHunterPhase::Lunge => {
                if state.timer.finished() {
                    state.phase = TideHunterPhase::Cooldown;
                    state.timer = Timer::from_seconds(0.55, TimerMode::Once);
                    state.timer.reset();
                }
            }
            TideHunterPhase::Cooldown => {
                if state.timer.finished() {
                    state.phase = TideHunterPhase::Stalk;
                }
            }
            TideHunterPhase::Stunned => {
                if state.timer.finished() {
                    state.phase = TideHunterPhase::Stalk;
                }
            }
        }
    }
}
```

### 新函数 `spawn_melee_hitbox` (boss.rs)

```rust
fn spawn_melee_hitbox(
    commands: &mut Commands,
    assets: &GameAssets,
    owner: Entity,
    pos: Vec2,
    dir: Vec2,
    damage: f32,
) {
    commands.spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation((pos + dir * 30.0).extend(40.0)),
            sprite: Sprite {
                color: Color::NONE,
                custom_size: Some(Vec2::splat(38.0)),
                ..default()
            },
            ..default()
        },
        Hitbox {
            owner: Some(owner),
            team: Team::Enemy,
            damage_kind: DamageKind::Enemy,
            size: Vec2::splat(38.0),
            damage,
            knockback: 260.0,
            can_crit: false,
            crit_chance: 0.0,
            crit_multiplier: 1.0,
        },
        Lifetime { timer: Timer::from_seconds(0.10, TimerMode::Once) },
        InGameEntity,
    ));
}
```

### 新系统 `tide_hunter_parry_check` (boss.rs)

```rust
pub fn tide_hunter_parry_check(
    player_q: Query<(&GlobalTransform, &DashState), (With<Player>, Without<Replicated>)>,
    mut boss_q: Query<(&Transform, &mut TideHunterState, &mut Flash), With<BossArchetype>>,
) {
    for (boss_tf, mut state, mut flash) in &mut boss_q {
        if !state.parry_window_active { continue; }
        let boss_pos = boss_tf.translation.truncate();
        for (player_tf, dash) in &player_q {
            let player_pos = player_tf.translation().truncate();
            if dash.active && boss_pos.distance(player_pos) < 65.0 {
                // 格挡成功
                state.phase = TideHunterPhase::Stunned;
                state.timer = Timer::from_seconds(1.6, TimerMode::Once);
                state.timer.reset();
                state.parry_window_active = false;
                flash.trigger(1.6); // 持续闪白表示硬直
                break;
            }
        }
    }
}
```

### `apply_damage_events` — 硬直增伤 (damage.rs)

```rust
tide_hunter_q: Query<&TideHunterState>,

// 在伤害应用后（health.current -= amount 之前）：
if let Ok(state) = tide_hunter_q.get(entity) {
    if state.phase == TideHunterPhase::Stunned {
        amount *= 2.0;
    }
}
```

### `spawn_boss` — 附加 TideHunterState (systems.rs)

```rust
if archetype == BossArchetype::TideHunter {
    commands.entity(id).insert(TideHunterState {
        phase: TideHunterPhase::Stalk,
        timer: Timer::from_seconds(0.1, TimerMode::Once),
        lunge_dir: Vec2::NEG_X,
        parry_window_active: false,
    });
}
```

---

## 步骤七：CubeCore — 子核心免疫

### `spawn_boss` — 子核心生成 (systems.rs)

```rust
if archetype == BossArchetype::CubeCore {
    // 主体附加护盾组件
    commands.entity(id).insert(BossCoreShield { cores_alive: 4 });

    // 生成 4 个子核心，均匀分布在 Boss 周围
    for i in 0..4u8 {
        let angle = i as f32 / 4.0 * std::f32::consts::TAU;
        let core_hp = 70.0_f32;
        let spawn_pos = Vec2::new(220.0, 0.0) + Vec2::new(angle.cos(), angle.sin()) * 85.0;
        commands.spawn((
            SpriteBundle {
                texture: assets.textures.white.clone(),
                transform: Transform::from_translation(spawn_pos.extend(44.0)),
                sprite: Sprite {
                    color: Color::srgb(1.0, 0.55, 0.75), // 比 CubeCore 亮一点
                    custom_size: Some(Vec2::splat(18.0)),
                    ..default()
                },
                ..default()
            },
            BossSubCore { boss_entity: id, orbit_angle: angle, orbit_speed: 0.55 },
            Health { current: core_hp, max: core_hp },
            EnemyKind(EnemyType::Boss),
            TeamMarker(Team::Enemy),
            Hurtbox { team: Team::Enemy, size: Vec2::splat(16.0) },
            Flash::new(0.0),
            Knockback(Vec2::ZERO),
            InGameEntity,
            Name::new("CubeCoreSubCore"),
        ));
    }
}
```

### 新系统 `boss_subcore_orbit` (boss.rs)

```rust
pub fn boss_subcore_orbit(
    time: Res<Time>,
    boss_q: Query<&Transform, (With<BossArchetype>, Without<BossSubCore>)>,
    mut core_q: Query<(&mut BossSubCore, &mut Transform), Without<BossArchetype>>,
) {
    for (mut core, mut tf) in &mut core_q {
        let Ok(boss_tf) = boss_q.get(core.boss_entity) else { continue };
        core.orbit_angle += core.orbit_speed * time.delta_seconds();
        let boss_pos = boss_tf.translation.truncate();
        let new_pos = boss_pos + Vec2::new(core.orbit_angle.cos(), core.orbit_angle.sin()) * 85.0;
        tf.translation.x = new_pos.x;
        tf.translation.y = new_pos.y;
    }
}
```

### 新系统 `boss_core_shield_update` (boss.rs)

子核心死亡后从世界中消失（`DeathEvent` 触发 `despawn`），该系统每帧统计存活子核心数量并更新护盾状态：

```rust
pub fn boss_core_shield_update(
    core_q: Query<&BossSubCore>,
    mut boss_q: Query<(Entity, &mut BossCoreShield, &mut Flash), With<BossArchetype>>,
) {
    for (boss_entity, mut shield, mut flash) in &mut boss_q {
        let alive = core_q.iter().filter(|c| c.boss_entity == boss_entity).count() as u8;
        if alive < shield.cores_alive && shield.cores_alive > 0 {
            // 子核心数减少，Boss 闪白提示
            flash.trigger(0.25);
        }
        shield.cores_alive = alive;
    }
}
```

### `apply_damage_events` — 护盾免疫 (damage.rs)

```rust
core_shield_q: Query<&BossCoreShield>,

// 在伤害无敌帧判断后：
if let Ok(shield) = core_shield_q.get(entity) {
    if shield.cores_alive > 0 {
        continue; // 完全免疫
    }
}
```

### Phase 转换时重生子核心 (`boss_phase_controller` 后处理)

监听 `BossPhaseChangeEvent`，若是 CubeCore 的 Phase2/3，召唤新一批子核心（数量减少：Phase2 = 2个，Phase3 = 2个）。在 `boss.rs` 新增系统 `boss_core_phase_respawn`：

```rust
pub fn boss_core_phase_respawn(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut phase_events: EventReader<BossPhaseChangeEvent>,
    boss_q: Query<(Entity, &Transform, &BossArchetype)>,
) {
    for ev in phase_events.read() {
        for (boss_entity, boss_tf, archetype) in &boss_q {
            if *archetype != BossArchetype::CubeCore { continue; }
            let count = 2u8;
            let boss_pos = boss_tf.translation.truncate();
            for i in 0..count {
                let angle = i as f32 / count as f32 * std::f32::consts::TAU;
                let core_hp = 70.0 + ev.phase as f32 * 20.0; // Phase2=90, Phase3=110
                let spawn_pos = boss_pos + Vec2::new(angle.cos(), angle.sin()) * 85.0;
                commands.spawn((
                    // 与 spawn_boss 中子核心 bundle 相同...
                    BossSubCore { boss_entity, orbit_angle: angle, orbit_speed: 0.65 },
                    // ...省略其余相同字段
                    InGameEntity,
                ));
            }
        }
    }
}
```

---

## 步骤八：玩家指引

每个 Boss 的机制都需要在**首次遭遇**时给出明确提示，避免玩家因不懂机制而产生挫败感。复用现有的 `TutorialFlags` + 提示条系统（`src/ui/tutorial.rs`）。

### 新增 TutorialFlag 条目

在 `TutorialFlags` 中追加 4 个字段：
```rust
pub boss_guardian_mechanic_shown: bool,
pub boss_mirror_warden_mechanic_shown: bool,
pub boss_tide_hunter_mechanic_shown: bool,
pub boss_cube_core_mechanic_shown: bool,
```

### 触发时机与文本

| Boss | 触发条件 | 提示文本 |
|------|---------|---------|
| Floor1Guardian | Boss 首次受到正面命中（减伤生效时） | "正面有防御！绕到侧面或背后打弱点" |
| MirrorWarden | 幻象首次生成（Phase2 首次传送后） | "找到真身！命中真身会闪光，幻象不会" |
| TideHunter | Boss 进入 WindupTelegraph 状态时 | "格挡机会！蓄力时用【空格】冲刺穿越" |
| CubeCore | Boss 首次触发护盾免疫（子核心全存活且有伤害被挡时） | "护盾！先摧毁周围的子核心" |

### 视觉强化（配合提示）

除文字提示外，还需要视觉上让机制"说话"：

**Floor1Guardian**
- 正面命中时，伤害数字颜色改为灰色（区别于正常的红/黄色伤害数字）
- Boss 的 `facing` 方向用颜色变化表示：朝玩家时 Boss 颜色略偏白（护盾激活感）

实现：在 `boss_guardian_facing_system` 中，根据 `facing.dot(dir_to_player)` 调整 Boss `Sprite.color` alpha 或亮度（`Color.set_l()` 或直接乘系数）。

**MirrorWarden**
- 幻象 50% 透明已有区分，但需确保幻象**不产生闪白效果**（`apply_damage_events` 中 `continue` 已处理）
- 真身命中时正常闪白，视觉反馈即为"这个是真的"

**TideHunter**
- WindupTelegraph 阶段：将 Boss `Sprite.color` 改为橙色（`Color::srgb(1.0, 0.55, 0.1)`），Lunge 后恢复原色
- Stunned 阶段：Boss `Sprite.color` 改为浅灰色，视觉上"被打懵了"
- 实现：在 `tide_hunter_state_machine` 的阶段切换处，通过 `commands.entity(boss_entity).insert(...)` 或直接 `Query<&mut Sprite>` 修改颜色

**CubeCore**
- 子核心存活时，主体 `Sprite.color` 加 alpha 遮罩效果（稍微变暗/变灰），表示"被保护"
- 子核心被击破时，主体短暂闪白（`boss_core_shield_update` 中已有 `flash.trigger(0.25)`）
- 子核心自身颜色比主体亮，作为明显的攻击目标标记

### 指引系统注册

在 `EnemySystemsPlugin` 中注册新系统 `boss_mechanic_hint_system`，读取 `BossPhaseChangeEvent` 和游戏内状态，在合适时机向 `TutorialNotification` 发送提示事件（复用第一阶段的提示条）。

---

## 步骤十：系统注册 (`src/gameplay/enemy/systems.rs` — `EnemySystemsPlugin`)

以下系统均添加 `.run_if(in_state(AppState::InGame).or_else(in_state(AppState::CoopGame).and_then(is_coop_authority)...))` 条件：

```
boss_guardian_facing_system
boss_decoy_system
tide_hunter_state_machine
tide_hunter_parry_check
boss_movement_override        (after update_enemy_ai)
boss_subcore_orbit
boss_core_shield_update
boss_core_phase_respawn
boss_mechanic_hint_system
```

---

## 验证方法

```bash
cargo check --quiet
cargo test --quiet
```

手动验证清单：
1. **Guardian**：正面攻击出现灰色伤害数字（低伤害）；首次触发时显示"正面有防御！"提示条
2. **MirrorWarden**：Phase2 传送后旧位置出现半透明幻象，幻象发弹但不闪白；首次幻象出现时显示"找到真身！"提示条
3. **TideHunter**：Phase1 无弹幕；蓄力时 Boss 变橙色；空格穿越后 Boss 变灰色静止 1.6 秒；首次蓄力时显示格挡提示
4. **CubeCore**：4 个小方块绕行，期间主体变暗/不受伤；首次免疫时显示"先摧毁子核心"提示；子核心逐一被消灭后主体开始受伤
5. `cargo test` 33 个测试全部通过

---

## 风险与注意事项

- `apply_damage_events` 新增 3 个 Query 参数，注意 Bevy Query 冲突（用 `Without<>` 过滤）
- `TideHunterPhase` 比较需要 `PartialEq`，已在枚举定义中加 derive
- `boss_subcore_orbit` 的 Query 需要区分主体和子核心：`Without<BossSubCore>` 和 `Without<BossArchetype>` 互相隔离
- CubeCore 子核心死亡路径：确保 `handle_enemy_death` 也处理 `EnemyType::Boss` 的小实体（子核心使用同一 EnemyType，需测试死亡逻辑）
- TideHunter Flash 触发（橙色预警）：`Flash` 组件目前只支持白色闪白，若需橙色需额外扩展或用 Sprite color 直接修改；简化方案是接受白色闪白即可

---

## Codex 执行简报

读取 `AGENTS.md` 和本文件，按步骤一→八顺序实现。每步完成后 `cargo check`。步骤四~七可并行实现（各自独立）。**不要修改**未列出的文件（如 pvp/、ui/ 等）。子核心体型 bundle 在步骤七中已完整列出，步骤七末尾的 `boss_core_phase_respawn` 需补全 bundle（参考步骤七中 spawn_boss 的子核心 spawn）。
