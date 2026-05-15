# PLANS.md — 美术素材接入

## 任务概述

将像素风格素材接入游戏，替换敌人/Boss 的纯色方块渲染，同时接入房间背景图。缺失素材自动 fallback 到纯色方块。

## 素材已就位

```
assets/textures/enemies/melee_chaser.png      (128×128 RGBA)
assets/textures/enemies/ranged_shooter.png    (128×128 RGBA)
assets/textures/bosses/floor1_guardian.png    (128×128 RGBA)
assets/textures/backgrounds/room_bg_default.jpg (2752×1536 RGB)
```

## 实施步骤

### Step 1: 扩展 TextureHandles（`src/core/assets.rs`）

在文件顶部添加：
```rust
use std::collections::HashMap;
use crate::gameplay::enemy::components::{EnemyType, BossArchetype};
```

在 `TextureHandles` struct 中添加字段：
```rust
pub enemy_sprites: HashMap<EnemyType, Handle<Image>>,
pub boss_sprites: HashMap<BossArchetype, Handle<Image>>,
pub room_background: Handle<Image>,
```

注意：`TextureHandles` 当前 derive 了 `Default`，添加 HashMap 字段后 Default 仍然可用（HashMap::default() 是空 map）。但 `room_background: Handle<Image>` 的 default 是弱句柄，这没问题因为我们在 `load_game_assets` 中会赋值。

在 `load_game_assets` 函数中，在构建 `TextureHandles` 之前添加加载逻辑：
```rust
let mut enemy_sprites = HashMap::new();
enemy_sprites.insert(EnemyType::MeleeChaser, asset_server.load("textures/enemies/melee_chaser.png"));
enemy_sprites.insert(EnemyType::RangedShooter, asset_server.load("textures/enemies/ranged_shooter.png"));

let mut boss_sprites = HashMap::new();
boss_sprites.insert(BossArchetype::Floor1Guardian, asset_server.load("textures/bosses/floor1_guardian.png"));

let room_background = asset_server.load("textures/backgrounds/room_bg_default.jpg");
```

在 `TextureHandles { ... }` 构造中添加这三个字段。

**不要**在 `check_assets_ready` 中添加这些新贴图的检查——它们是可选的，缺失不应阻塞加载。

### Step 2: 修改敌人生成（`src/gameplay/enemy/systems.rs`）

首先，给 `spawn_enemy_with_elite_scale` 函数添加一个新参数 `use_sprite_textures: bool`。

在调用处 `spawn_enemy`（约 line 680）中，从 `data.balance.use_sprite_textures` 取值后传入。

在 `spawn_enemy_with_elite_scale` 函数中（约 line 792），将：
```rust
let mut entity = commands.spawn((
    SpriteBundle {
        texture: assets.textures.white.clone(),
        transform,
        sprite: Sprite {
            color,
            custom_size: Some(Vec2::splat(sprite_size)),
            ..default()
        },
        ..default()
    },
```

改为：
```rust
let (texture, sprite_color) = if use_sprite_textures {
    if let Some(tex) = assets.textures.enemy_sprites.get(&enemy_type) {
        let tint = if is_elite && enemy_type != EnemyType::Boss {
            Color::srgb(1.0, 0.92, 0.7)
        } else {
            Color::WHITE
        };
        (tex.clone(), tint)
    } else {
        (assets.textures.white.clone(), color)
    }
} else {
    (assets.textures.white.clone(), color)
};
let mut entity = commands.spawn((
    SpriteBundle {
        texture,
        transform,
        sprite: Sprite {
            color: sprite_color,
            custom_size: Some(Vec2::splat(sprite_size)),
            ..default()
        },
        ..default()
    },
```

### Step 3: 修改 Boss 生成（`src/gameplay/enemy/systems.rs`）

在 `spawn_boss` 函数中（约 line 964），将：
```rust
let id = commands
    .spawn((
        SpriteBundle {
            texture: assets.textures.white.clone(),
            transform: Transform::from_translation(Vec3::new(220.0, 0.0, 45.0)),
            sprite: Sprite {
                color: boss::boss_color(archetype),
                custom_size: Some(Vec2::splat(sprite_size)),
                ..default()
            },
            ..default()
        },
```

改为：
```rust
let use_textures = data.balance.use_sprite_textures;
let (boss_texture, boss_sprite_color) = if use_textures {
    if let Some(tex) = assets.textures.boss_sprites.get(&archetype) {
        (tex.clone(), Color::WHITE)
    } else {
        (assets.textures.white.clone(), boss::boss_color(archetype))
    }
} else {
    (assets.textures.white.clone(), boss::boss_color(archetype))
};
let id = commands
    .spawn((
        SpriteBundle {
            texture: boss_texture,
            transform: Transform::from_translation(Vec3::new(220.0, 0.0, 45.0)),
            sprite: Sprite {
                color: boss_sprite_color,
                custom_size: Some(Vec2::splat(sprite_size)),
                ..default()
            },
            ..default()
        },
```

### Step 4: 修改 Flash 效果（`src/gameplay/effects/flash.rs`）

将 line 34 的：
```rust
sprite.color = Color::WHITE;
```

改为：
```rust
sprite.color = Color::srgb(2.5, 2.5, 2.5);
```

### Step 5: 接入房间背景图（`src/gameplay/map/tiles.rs`）

在 `spawn_room_tiles` 函数中（line 38-51），将房间地板的 SpriteBundle 改为：
```rust
commands.spawn((
    SpriteBundle {
        texture: assets.textures.room_background.clone(),
        sprite: Sprite {
            color: Color::WHITE,
            custom_size: Some(Vec2::new(ROOM_HALF_WIDTH * 2.0, ROOM_HALF_HEIGHT * 2.0)),
            ..default()
        },
        ..default()
    },
    RoomTiles,
    InGameEntity,
    Name::new("RoomFloor"),
));
```

### Step 6: 添加配置回退开关

在 `src/data/definitions.rs` 的 `GameBalanceConfig` struct（line 195）末尾，在 `elite_gold_bonus: u32,` 之后添加：
```rust
#[serde(default = "default_use_sprite_textures")]
pub use_sprite_textures: bool,
```

在同文件中（struct 外部）添加辅助函数：
```rust
fn default_use_sprite_textures() -> bool { true }
```

在 `assets/configs/game_balance.ron` 中，在 `elite_gold_bonus: 18,` 之后添加：
```
use_sprite_textures: true,
```

注意：`GameBalanceConfig` 在 `src/data/registry.rs` 中被加载为 `data.balance`。确认 `data.balance.use_sprite_textures` 可以在 `spawn_boss` 和 `spawn_enemy` 中访问。`spawn_boss` 已有 `data: &GameDataRegistry` 参数。`spawn_enemy`（line 680）也有 `data: &GameDataRegistry` 参数。

## Affected files

- `src/core/assets.rs` (modified)
- `src/gameplay/enemy/systems.rs` (modified)
- `src/gameplay/effects/flash.rs` (modified)
- `src/gameplay/map/tiles.rs` (modified)
- `src/data/definitions.rs` (modified)
- `assets/configs/game_balance.ron` (modified)

## Validation

```bash
cargo check --quiet
cargo test --quiet
```
