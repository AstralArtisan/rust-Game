# 修复计划：UI清理 + 事件房体验 + 进门位置

## Context

用户游玩后发现4个体验问题：铭文UI残留（系统已废弃但UI未清理）、事件房背景黑屏风格不符、事件房有概率连续触发、进门刷新位置不固定且可能刷在怪脸上。

---

## 问题 1: 铭文UI清理

**根因**: `RunePlugin` 已是空实现（`fn build(&self, _app: &mut App) {}`），铭文系统功能已废弃，但 HUD 中仍有铭文槽位 UI 和更新系统。

**影响文件**:
- `src/ui/hud.rs`
- `src/ui/mod.rs`

**改动**:

### 1a. `src/ui/hud.rs`

1. 删除组件定义（约第 64-67 行）：
   ```rust
   #[derive(Component, Debug, Clone, Copy)]
   pub struct RuneHudSlot(pub RuneSlot);
   #[derive(Component, Debug, Clone, Copy)]
   pub struct RuneHudText(pub RuneSlot);
   ```

2. 删除 `setup_hud()` 中铭文 UI 节点（约第 322-360 行）：整个 `runes.spawn(widgets::title_text(&assets, "铭文", 15.0))` 及其子节点块。

3. 删除 `update_rune_and_curse_ui` 函数中铭文相关的查询和更新逻辑（保留诅咒状态更新，只删除铭文部分）。

### 1b. `src/ui/mod.rs`

删除 `hud::update_rune_and_curse_ui` 的注册（如果铭文逻辑被完全移除后该函数仍有诅咒逻辑则保留，否则删除）。

**注意**: 保留 `src/gameplay/rune/` 目录下的数据结构（`RuneLoadout`、`RuneSlot` 等），因为可能被其他地方引用。只清理 UI 层。

---

## 问题 2: 事件房流程重设计

**根因**: 当前非战斗事件房进入后立即切换到 `AppState::EventRoom` 弹出选择 UI，没有"交互过程"，且完成后直接返回游戏，没有选门流程。用户期望：进入事件房 → 有交互过程 → 完成后像战斗房一样选门离开。

**新流程设计**:
```
进入事件房
  → RoomState::Locked（房间锁定，门关闭）
  → 在游戏世界中显示事件交互提示（按 E 键触发事件）
  → 玩家走近事件触发点按 E → 弹出 AppState::EventRoom UI
  → 玩家选择完成 → RoomState::Cleared（门开启）
  → 玩家自由选门离开（与战斗房完全一致）
```

**影响文件**:
- `src/gameplay/event_room/mod.rs`
- `src/ui/event_room.rs`

**改动**:

### 2a. 进入事件房时先锁定房间，不立即弹 UI

`src/gameplay/event_room/mod.rs` 中 `select_and_spawn_event` 函数，对非战斗事件的处理（约第 267-275 行）：

**当前**：
```rust
EventType::Gambler | ... => {
    configure_non_combat_event(...);
    next_state.set(AppState::EventRoom);  // 立即弹UI
}
```

**改为**：
```rust
EventType::Gambler | ... => {
    configure_non_combat_event(...);
    *room_state = RoomState::Locked;  // 锁定房间
    active.interaction_ready = true;  // 标记等待玩家交互
    // 不切换状态，留在 InGame
}
```

在 `ActiveEvent` 结构体中新增字段 `pub interaction_ready: bool`，默认 `false`。

### 2b. 在游戏世界中生成事件交互提示实体

在 `select_and_spawn_event` 中，非战斗事件触发时，在房间中心生成一个交互提示实体：

```rust
commands.spawn((
    Text2dBundle {
        text: Text::from_section(
            format!("【{}】\n按 E 交互", event_type.title()),
            TextStyle { font_size: 20.0, color: accent_color, ... }
        ),
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        ..default()
    },
    EventInteractPrompt,
    InGameEntity,
));
```

新增 `EventInteractPrompt` 组件标记。`accent_color` 根据事件类型选择（金/紫/红/绿/蓝/橙）。

### 2c. 新增玩家靠近交互触发系统

新增系统 `event_interact_system`，在 `AppState::InGame` 下运行：

```rust
fn event_interact_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut active: ResMut<ActiveEvent>,
    mut next_state: ResMut<NextState<AppState>>,
    player_q: Query<&GlobalTransform, With<Player>>,
    prompt_q: Query<(Entity, &GlobalTransform), With<EventInteractPrompt>>,
    mut commands: Commands,
) {
    if !active.interaction_ready { return; }
    // 玩家靠近提示点（80px 内）且按 E 键
    if keyboard.just_pressed(KeyCode::KeyE) {
        // 销毁提示实体
        for (e, _) in &prompt_q { commands.entity(e).despawn_recursive(); }
        active.interaction_ready = false;
        next_state.set(AppState::EventRoom);  // 此时才弹 UI
    }
}
```

### 2d. 事件完成后设置 Cleared，让玩家选门

`event_room_input` 中所有退出路径都必须设置 `RoomState::Cleared`，让门开启：

- `EventInputOutcome::Complete` 分支（约第 351 行）：已有 `*room_state = RoomState::Cleared`，保持不变
- `Esc` 键放弃分支（约第 319 行）：添加 `*room_state = RoomState::Cleared`
- `EventChoicePayload::Leave` 分支：添加 `*room_state = RoomState::Cleared`

这样无论玩家选择完成事件还是放弃，房间都会解锁，门变金色可通行。

### 2e. 改进事件 UI 样式

`src/ui/event_room.rs` 中：
- `scrim_node(0.62)` → `scrim_node(0.40)`（降低遮罩，让背景隐约可见）
- 面板左侧添加 8px 彩色竖条（根据事件类型颜色）
- 标题前加符号前缀（赌徒→`◈`，诅咒→`☠`，血契→`♦`，宝箱→`✦`，治愈泉→`✿`，商贩→`⚙`）

---

## 问题 3: 事件房连续触发

**根因**: 同一事件房可能在某些条件下被重复触发。

**影响文件**: `src/gameplay/event_room/mod.rs`

**改动**: 在 `select_and_spawn_event` 中，只要 `active.room == Some(current_room.0)` 就直接返回，不再检查 resolved 状态：

```rust
if active.room == Some(current_room.0) {
    return;
}
```

---

## 问题 4: 进门刷新位置固定 + 敌人生成保护区域

**根因**:
- `player_spawn_position` 根据进入方向动态计算位置（左进→左侧，右进→右侧，上进→上方，下进→下方）
- 敌人生成点硬编码，无最小距离检查，从下方进入时玩家可能直接刷在敌人旁边（距离仅 36px）

**影响文件**:
- `src/gameplay/map/transitions.rs`
- `src/gameplay/enemy/spawner.rs`（或 `systems.rs`）

**改动**:

### 4a. 固定进门位置到左侧

`src/gameplay/map/transitions.rs` 中 `player_spawn_position` 函数：

```rust
fn player_spawn_position(_entry_from: Direction, z: f32, y_offset: f32) -> Vec3 {
    // 始终在房间左侧固定位置
    Vec3::new(-ROOM_HALF_WIDTH * 0.6, y_offset, z)
}
```

### 4b. 敌人生成点添加玩家保护区域

在 `src/gameplay/enemy/spawner.rs`（或 `systems.rs`）的 `spawn_room_enemies` 中，添加过滤逻辑：

玩家固定在 `(-ROOM_HALF_WIDTH * 0.6, 0)` = `(-312, 0)`，设置保护半径 **120px**。

```rust
let player_spawn = Vec2::new(-ROOM_HALF_WIDTH * 0.6, 0.0);
let safe_points: Vec<Vec2> = points.iter()
    .filter(|&&p| p.distance(player_spawn) >= 120.0)
    .copied()
    .collect();
// 使用 safe_points 替代 points
```

如果过滤后点数不足，将不足的点替换为距离玩家最远的备用点（房间右侧区域）。

---

## 影响文件汇总

| 文件 | 改动 |
|------|------|
| `src/ui/hud.rs` | 删除铭文 UI 节点和组件 |
| `src/ui/mod.rs` | 删除铭文 update 系统注册（如适用） |
| `src/ui/event_room.rs` | 降低 scrim 透明度，添加彩色边框和符号前缀 |
| `src/gameplay/event_room/mod.rs` | 新增 `interaction_ready` 字段、交互提示生成、`event_interact_system`、防重复触发加强、标题符号 |
| `src/gameplay/map/transitions.rs` | 固定进门位置到左侧 |
| `src/gameplay/enemy/spawner.rs` 或 `systems.rs` | 添加玩家保护区域过滤（120px） |

---

## 验证

```bash
cargo check --quiet
cargo test --quiet
```

手动验证：
1. HUD 中不再显示铭文槽位
2. 进入事件房后房间锁定，中央出现彩色交互提示文字
3. 走近提示按 E → 弹出事件选择 UI（背景半透明可见游戏世界，面板有彩色边框）
4. 选择完成 → 返回游戏，门变金色，可自由选门离开
5. 同一事件房不会重复触发
6. 无论从哪个方向进门，玩家始终出现在房间左侧
7. 进门后附近 120px 内无敌人生成
