# Phase 9 设计改进计划

## Context

用户在游玩测试后提出 5 项设计改进需求：
1. 事件房 Esc 退出后可重新交互（已实现，确认无需改动）
2. 升级时提供"回血或强化"选择（参考 pre-augment-system tag 的 HealOrBuff UI 布局）
3. 小怪头顶加血条，提升战斗信息反馈
4. 精英房重新设计，与普通房拉开差距
5. 精英词缀标签乱码修复（Text2dBundle 未指定中文字体）

改动 2（小怪血条）和改动 3（精英房）已在上一轮 Codex 中实现，本轮只需实现改动 1（升级UI重构）和改动 4（词缀标签字体修复）。

---

## 改动 1：升级 UI 重构为"回血或强化"双栏布局

### 当前状态
`src/ui/levelup_select.rs`：升级时显示 3 个随机属性卡片，玩家按 1/2/3 选择。
`src/gameplay/progression/experience.rs`：`handle_levelup_event` 从 7 个属性中随机选 3 个（含上轮新增的 RecoverHealth）。

### 参考设计
`pre-augment-system` tag 的 `src/ui/reward_select.rs` 中 `HealOrBuff` 模式：
- 左栏：固定"回血"按钮（按 1），显示恢复量
- 右栏：3 个强化选项（按 2/3/4）

### 方案

**文件：`src/ui/levelup_select.rs`**

重构 `setup_levelup_ui`，改为双栏布局：

```
┌─────────────────────────────────────────────────────┐
│           升级！ Lv.X                                │
│  选择回血恢复状态，或选择一项属性提升                  │
├──────────────────┬──────────────────────────────────┤
│   回血           │        属性强化                    │
│ ┌──────────────┐ │ ┌──────────────────────────────┐ │
│ │ 1. 回血      │ │ │ 2. 攻击力 +3               │ │
│ │ 恢复 XX 生命 │ │ │ 提升近战和远程攻击伤害      │ │
│ │ 稳住当前状态 │ │ └──────────────────────────────┘ │
│ │ 后继续推进   │ │ ┌──────────────────────────────┐ │
│ └──────────────┘ │ │ 3. 生命上限 +15             │ │
│                  │ │ 提升最大生命值并回复等量HP   │ │
│                  │ └──────────────────────────────┘ │
│                  │ ┌──────────────────────────────┐ │
│                  │ │ 4. 暴击率 +5%               │ │
│                  │ │ 提升暴击概率                 │ │
│                  │ └──────────────────────────────┘ │
└──────────────────┴──────────────────────────────────┘
```

具体改动：
1. `setup_levelup_ui` 改为双栏布局（参考 `reward_select.rs` 行132-200 的 HealOrBuff 布局）
2. 左栏：固定回血按钮，`LevelUpButton { index: 0 }`，显示回血量
3. 右栏：3 个属性强化卡片，`LevelUpButton { index: 1/2/3 }`
4. 回血量计算：使用 `heal_amount` 函数（已存在于 `src/gameplay/rewards/apply.rs:127`），需要导入
5. 需要新增系统参数：`health_q: Query<&Health, With<Player>>`, `floor: Option<Res<FloorNumber>>`, `data: Option<Res<GameDataRegistry>>`

保留 `LevelUpStat::RecoverHealth(f32)` 变体（上轮已添加）。

修改 `levelup_input`：
- 按键 1：选择回血（index 0）
- 按键 2/3/4：选择 3 个属性强化之一（index 1/2/3）
- 回血处理已在上轮实现：`LevelUpStat::RecoverHealth(pct) => { health.current = (health.current + health.max * pct).min(health.max); }`

**文件：`src/gameplay/progression/experience.rs`**

修改 `handle_levelup_event`：
- `choices.options` 改为 4 个选项：第 0 个固定为回血，第 1-3 个为随机属性（从 6 个非回血属性中选）
- 回血选项：`LevelUpOption { label: "回血".to_string(), description: format!("恢复 {:.0} 生命\n稳住当前状态后继续推进", heal_value), apply: LevelUpStat::RecoverHealth(heal_value) }`
- 回血量用绝对值而非百分比：`heal_value = heal_amount(&scaling, health.max, floor_number)`
- 需要新增系统参数：`health_q: Query<&Health, With<Player>>`, `floor: Option<Res<FloorNumber>>`, `data: Option<Res<GameDataRegistry>>`
- 从 `all_stats` 中移除 `RecoverHealth` 条目（回血不再随机出现，而是固定在第 0 位）

---

## 改动 4：精英词缀标签乱码修复

### 根因
`src/gameplay/enemy/systems.rs` 精英词缀的 `Text2dBundle` 使用了 `TextStyle { font_size: 18.0, color: ..., ..default() }`，`font` 字段使用了 Bevy 默认字体（不支持中文），导致中文词缀名显示为乱码。

### 方案

**文件：`src/gameplay/enemy/systems.rs`**

在精英词缀标签生成代码中，为所有 `TextStyle` 添加 `font: assets.font.clone()`。

描边文字（4个黑色阴影副本）：
```rust
TextStyle {
    font: assets.font.clone(),
    font_size: 18.0,
    color: Color::srgba(0.0, 0.0, 0.0, 0.9),
}
```

主标签：
```rust
TextStyle {
    font: assets.font.clone(),
    font_size: 18.0,
    color: label_color,
}
```

`assets` 在 `spawn_enemy_with_elite_scale` 函数中已有参数 `assets: &GameAssets`。✓

---

## Affected files

| 文件 | 改动类型 | 描述 |
|------|---------|------|
| `src/ui/levelup_select.rs` | modified | 重构为双栏布局（回血+强化） |
| `src/gameplay/progression/experience.rs` | modified | handle_levelup_event 生成 4 选项（1回血+3属性） |
| `src/gameplay/enemy/systems.rs` | modified | 词缀标签字体修复（添加 font: assets.font.clone()） |

## 验证步骤

```bash
cargo check --quiet
cargo test --quiet
```

### 手动测试
1. **升级回血**：升级时看到双栏 UI → 左边"回血"按钮显示恢复量 → 按 1 回血 → 按 2/3/4 选属性强化
2. **精英词缀标签**：精英怪头顶显示中文词缀名（迅捷/分裂/护盾等），不再乱码
