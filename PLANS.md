# Phase 6: 警告清理 + 精英词缀 HUD + 数值外部化

## Context

Phase 5 完成了新怪物、精英词缀和 TideHunter 调整。当前代码有 91 个编译警告（废弃 API、未使用代码、多余 mut），精英词缀对玩家不可见（无 HUD 提示），大量平衡数值硬编码在 Rust 源码中无法热调。

Phase 6 解决三个问题：代码质量、精英可读性、数值可调性。

分三个子阶段，每个可独立编译测试。执行顺序：6a → 6b → 6c。

---

## Sub-Phase 6a: 编译警告清零 + 死代码清理

### 目标

消除全部 91 个编译警告，零行为变更。

### Affected files

| 文件 | 操作 |
|------|------|
| `src/core/audio.rs` | 删除未使用 import `GameAssets`（第 6 行） |
| `src/gameplay/effects/particles.rs` | 删除未使用 import `GameDataRegistry`（第 5 行） |
| `src/ui/augment_select.rs` | 删除未使用 import `GameDataRegistry`（第 4 行） |
| `src/prelude.rs` | 文件顶部加 `#![allow(unused_imports)]` |
| `src/coop/ui.rs` | ReceivedCharacter → KeyboardInput（第 309、323 行）；删除多余 mut（第 591 行） |
| `src/pvp/ui.rs` | ReceivedCharacter → KeyboardInput（第 133、163 行） |
| `src/coop/runtime.rs` | 删除 18 处多余 mut |
| `src/core/save.rs` | 删除多余 mut（第 183 行） |
| `src/gameplay/drops/mod.rs` | 删除未使用变量 `closest`（第 267、273 行） |
| `src/ui/hud.rs` | 删除死代码：`update_minimap`、`room_color`、`MinimapRoomNode`、`MinimapDynamic` |
| `src/utils/easing.rs` | 3 个未使用函数加 `#[allow(dead_code)]` |
| `src/utils/rng.rs` | `reseed` 方法加 `#[allow(dead_code)]` |
| `src/utils/timers.rs` | `tick_timer` 加 `#[allow(dead_code)]` |
| `src/ui/widgets.rs` | `info_color`、`panel_node_with_padding` 加 `#[allow(dead_code)]` |

其余 dead_code 警告（player/components.rs 的字段、session_core 的函数等）统一加 `#[allow(dead_code)]`，不删除（可能未来使用）。

### 详细改动

#### ReceivedCharacter 替换

`src/coop/ui.rs` 和 `src/pvp/ui.rs` 中的 `EventReader<ReceivedCharacter>` 替换为 `EventReader<KeyboardInput>`：

```rust
// 旧：
for ev in chars.read() {
    for c in ev.char.chars() { ... }
}
// 新：
use bevy::input::keyboard::Key;
for ev in key_events.read() {
    if !ev.state.is_pressed() { continue; }
    if let Key::Character(ref s) = ev.logical_key {
        for c in s.chars() { ... }
    }
}
```

保持原有字符过滤逻辑不变。

#### Minimap 死代码删除

删除 `src/ui/hud.rs` 中：
- `MinimapRoomNode` 和 `MinimapDynamic` 结构体定义
- `update_minimap` 函数（约 100 行）
- `room_color` 辅助函数

保留 `MinimapRoot` 组件（仍在 `setup_hud` 中使用）。

### 验证

```bash
cargo check 2>&1 | grep "warning:" | wc -l  # 目标：0
cargo test --quiet  # 44 passed
```

---

## Sub-Phase 6b: 精英词缀浮动标签

### 目标

精英怪头顶显示词缀名称标签，让玩家能识别精英类型并调整策略。

### Affected files

| 文件 | 操作 |
|------|------|
| `src/gameplay/enemy/components.rs` | 新增 `EliteAffix::label()` 和 `EliteAffix::color()` 方法 + `EliteAffixLabel` 组件 |
| `src/gameplay/enemy/systems.rs` | spawn_enemy 中为精英生成浮动标签子实体；新增 `update_elite_label_system` |

### 详细改动

#### 1. `src/gameplay/enemy/components.rs`

为 `EliteAffix` 枚举添加方法：

```rust
impl EliteAffix {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Swift => "迅捷",
            Self::Splitting => "分裂",
            Self::Shielded => "护盾",
            Self::Vampiric => "吸血",
            Self::Berserk => "狂暴",
            Self::Teleporting => "闪现",
        }
    }
    pub fn color(&self) -> Color {
        match self {
            Self::Swift => Color::srgb(0.3, 0.9, 1.0),
            Self::Splitting => Color::srgb(0.5, 1.0, 0.5),
            Self::Shielded => Color::srgb(0.7, 0.7, 1.0),
            Self::Vampiric => Color::srgb(1.0, 0.3, 0.3),
            Self::Berserk => Color::srgb(1.0, 0.5, 0.0),
            Self::Teleporting => Color::srgb(0.8, 0.4, 1.0),
        }
    }
}
```

新增标记组件：
```rust
#[derive(Component)]
pub struct EliteAffixLabel;
```

#### 2. `src/gameplay/enemy/systems.rs`

在 `spawn_enemy` 中，插入 `EliteAffixMarker` 后，生成一个 `Text2dBundle` 子实体：

```rust
if is_elite {
    // ... existing affix assignment ...
    let affix = /* the assigned affix */;
    let label_entity = commands.spawn((
        Text2dBundle {
            text: Text::from_section(
                affix.label(),
                TextStyle {
                    font_size: 12.0,
                    color: affix.color(),
                    ..default()
                },
            ),
            transform: Transform::from_translation(Vec3::new(0.0, 20.0, 10.0)),
            ..default()
        },
        EliteAffixLabel,
    )).id();
    commands.entity(enemy_entity).add_child(label_entity);
}
```

标签作为子实体自动跟随敌人移动，无需额外更新系统。

### 验证

```bash
cargo check --quiet && cargo test --quiet
```

---

## Sub-Phase 6c: 奖励数值外部化

### 目标

将 `src/gameplay/rewards/apply.rs` 中硬编码的楼层奖励曲线移到 `assets/configs/rewards.ron`，使数值可热调。

### 范围限定

只外部化奖励系统的楼层增益曲线（7 种属性 × 4 楼层 = 28 个数值），不动武器精通、技能常量等（留给后续迭代）。

### Affected files

| 文件 | 操作 |
|------|------|
| `src/data/definitions.rs` | 新增 `RewardScalingConfig` 结构体 |
| `src/data/loaders.rs` | `default_registry` 中填充默认值 |
| `assets/configs/rewards.ron` | 新增 `scaling` 段 |
| `src/gameplay/rewards/apply.rs` | 7 个 gain 函数改为读取 `GameDataRegistry` |

### 详细改动

#### 1. `src/data/definitions.rs`

新增：
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct FloorGains {
    pub floor_1: f32,
    pub floor_2: f32,
    pub floor_3: f32,
    pub floor_4: f32,
}

impl FloorGains {
    pub fn get(&self, floor: u32) -> f32 {
        match floor {
            1 => self.floor_1,
            2 => self.floor_2,
            3 => self.floor_3,
            _ => self.floor_4,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RewardScalingConfig {
    pub attack_speed_s: FloorGains,
    pub attack_power: FloorGains,
    pub max_health: FloorGains,
    pub dash_cooldown_s: FloorGains,
    pub lifesteal: FloorGains,
    pub crit_chance: FloorGains,
    pub move_speed: FloorGains,
    pub heal_base: FloorGains,
    pub heal_hp_fraction: f32,
}
```

在 `RewardsConfig` 中新增字段 `pub scaling: RewardScalingConfig`。

#### 2. `assets/configs/rewards.ron`

在现有内容中新增 `scaling` 段：
```ron
scaling: RewardScalingConfig(
    attack_speed_s: FloorGains(floor_1: 0.04, floor_2: 0.06, floor_3: 0.07, floor_4: 0.08),
    attack_power: FloorGains(floor_1: 4.0, floor_2: 5.0, floor_3: 6.0, floor_4: 7.0),
    max_health: FloorGains(floor_1: 20.0, floor_2: 24.0, floor_3: 28.0, floor_4: 32.0),
    dash_cooldown_s: FloorGains(floor_1: 0.08, floor_2: 0.10, floor_3: 0.12, floor_4: 0.14),
    lifesteal: FloorGains(floor_1: 3.0, floor_2: 4.0, floor_3: 5.0, floor_4: 6.0),
    crit_chance: FloorGains(floor_1: 0.03, floor_2: 0.04, floor_3: 0.05, floor_4: 0.06),
    move_speed: FloorGains(floor_1: 18.0, floor_2: 24.0, floor_3: 30.0, floor_4: 36.0),
    heal_base: FloorGains(floor_1: 24.0, floor_2: 30.0, floor_3: 36.0, floor_4: 42.0),
    heal_hp_fraction: 0.22,
),
```

#### 3. `src/data/loaders.rs`

`default_registry` 中为 `RewardScalingConfig` 填充与上述相同的默认值。

#### 4. `src/gameplay/rewards/apply.rs`

7 个 `*_gain` 函数（约第 97-168 行）从 match floor 硬编码改为：
```rust
// 旧：
fn attack_speed_gain_s(floor: u32) -> f32 {
    match floor { 1 => 0.04, 2 => 0.06, 3 => 0.07, _ => 0.08 }
}
// 新：
fn attack_speed_gain_s(scaling: &RewardScalingConfig, floor: u32) -> f32 {
    scaling.attack_speed_s.get(floor)
}
```

调用处传入 `&data.rewards.scaling`。

### 验证

```bash
cargo check --quiet && cargo test --quiet
```

手动验证：运行游戏，确认奖励数值与改动前一致。

---

## 实施顺序

```
6a (警告清零) → 6b (精英标签) → 6c (数值外部化)
```
