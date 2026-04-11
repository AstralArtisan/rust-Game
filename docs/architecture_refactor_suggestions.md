# 架构修改建议

> 基于 2026-04-11 代码审查，记录当前架构中发现的问题和重构建议。
> 这些问题不影响运行，但会增加维护成本，建议后续迭代中逐步修复。

---

## 问题 1: 铭文系统（Rune）残留代码未清理

**严重程度**：高——设计已移除铭文，但代码和配置完全保留

**现象**：
项目中仍有 14 个源文件和 1 个配置文件引用铭文系统：

| 文件 | 残留内容 |
|------|----------|
| `src/gameplay/rune/mod.rs` | RunePlugin（空壳 Plugin） |
| `src/gameplay/rune/data.rs` | RuneId（31 种）、RuneSlot、RuneTier、RuneLoadout 完整定义 |
| `src/gameplay/session_core/mod.rs` | BlessingOffer 引用 RuneId/RuneSlot/RuneTier，祝福祠堂逻辑 |
| `src/gameplay/rewards/systems.rs` | 祝福祠堂生成逻辑引用铭文 |
| `src/gameplay/player/systems.rs` | 玩家生成时初始化 RuneLoadout 组件 |
| `src/data/definitions.rs` | RunesConfig 结构定义 |
| `src/data/loaders.rs` | 加载 runes.ron 的逻辑 |
| `src/data/registry.rs` | GameDataRegistry 包含 `runes: RunesConfig` 字段 |
| `src/ui/reward_select.rs` | 祝福选择 UI 引用铭文数据 |
| `src/ui/hud.rs` | HUD 引用铭文 |
| `src/app.rs` | 注册 RunePlugin |
| `src/gameplay/mod.rs` | 声明 `pub mod rune` |
| `src/gameplay/augment/data.rs` | 可能有交叉引用 |
| `assets/configs/runes.ron` | 31 个铭文的完整配置文件 |

**建议**：
1. 删除 `src/gameplay/rune/` 整个目录
2. 删除 `assets/configs/runes.ron`
3. 从 `GameDataRegistry` 移除 `runes` 字段
4. 从 `data/definitions.rs` 移除 `RunesConfig`
5. 从 `data/loaders.rs` 移除铭文加载逻辑
6. 清理 `session_core`、`rewards/systems.rs`、`player/systems.rs`、UI 中的铭文引用
7. 从 `app.rs` 移除 `RunePlugin` 注册
8. 重新审视祝福祠堂逻辑——如果铭文移除后祝福祠堂也不再需要，一并清理

**注意**：这是一个涉及 14 个文件的清理任务，建议单独开一个分支处理，完成后跑 `cargo check` + `cargo test` 验证。

---

## 问题 2: 插件注册位置不一致

**严重程度**：中

**现象**：
`AugmentPlugin`、`RunePlugin`、`CursePlugin` 在 `src/app.rs:51-55` 的 `GamePlugin` 中注册：

```rust
// app.rs — GamePlugin::build()
.add_plugins((
    crate::gameplay::augment::AugmentPlugin,
    crate::gameplay::rune::RunePlugin,
    crate::gameplay::curse::CursePlugin,
));
```

而它们是 `gameplay/` 的子模块，其他同级模块（MapPlugin、PlayerPlugin、CombatPlugin 等）都在 `gameplay/mod.rs` 的 `GameplayPlugin` 中注册。

**影响**：
- 破坏了层级一致性：gameplay 子模块跳过 GameplayPlugin 直接挂在顶层
- 新维护者看 `GameplayPlugin` 时会以为这三个模块没有被注册
- 后续如果要统一管理 gameplay 插件的启停，这三个会被遗漏

**建议**：
将这三个插件移入 `gameplay/mod.rs` 的 `GameplayPlugin::build()` 中：

```rust
// gameplay/mod.rs
impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            map::MapPlugin,
            progression::ProgressionPlugin,
            combat::CombatPlugin,
            player::PlayerPlugin,
            skills::SkillsPlugin,
            enemy::EnemyPlugin,
            rewards::RewardsPlugin,
            effects::EffectsPlugin,
            puzzle::PuzzlePlugin,
            event_room::EventRoomPlugin,
            shop::ShopPlugin,
            drops::DropPlugin,
            augment::AugmentPlugin,  // 从 app.rs 移入
            curse::CursePlugin,      // 从 app.rs 移入
        ))
        // ...
    }
}
```

---

## 问题 3: EventRoom UI 系统泄漏到顶层

**严重程度**：中

**现象**：
`app.rs:44-49` 直接注册了事件房 UI 的 setup/cleanup 系统：

```rust
// app.rs — GamePlugin::build()
.add_systems(OnEnter(AppState::EventRoom), ui::event_room::setup_event_room_ui)
.add_systems(OnExit(AppState::EventRoom), ui::event_room::cleanup_event_room_ui)
```

**影响**：
- 事件房有自己的 `EventRoomPlugin`，这两个系统应该在那里注册
- 散落在 `GamePlugin` 中增加了理解成本

**建议**：
将这两个系统移入 `EventRoomPlugin::build()` 或 `UiPlugin::build()` 中，从 `app.rs` 删除。

---

## 问题 4: TeamMarker 重复定义

**严重程度**：低

**现象**：
两个文件各自定义了相同的 `TeamMarker`：
- `src/gameplay/player/components.rs:600` — `pub struct TeamMarker(pub Team);`
- `src/gameplay/enemy/components.rs:106` — `pub struct TeamMarker(pub Team);`

两处都标了 `#[allow(dead_code)]`。

**影响**：
- 两个同名但不同类型（不同模块路径），容易混淆
- 如果未来需要统一使用 TeamMarker，会产生歧义

**建议**：
统一到 `src/gameplay/combat/components.rs` 中（Team 枚举已在此定义），删除 player 和 enemy 中的重复定义。如果确认未使用，直接删除两处。

---

## 问题 5: 大量模块级 `#![allow(dead_code)]`

**严重程度**：低

**现象**：
以下文件在模块顶部使用了 `#![allow(dead_code)]`，抑制整个模块的未使用代码警告：
- `src/gameplay/session_core/mod.rs`
- `src/gameplay/player/components.rs`（部分项）
- `src/gameplay/enemy/components.rs`
- `src/core/events.rs`
- `src/data/registry.rs`

**影响**：
- 掩盖了真正未使用的代码（比如铭文相关的残留）
- 新增代码如果未被使用，也不会收到编译器警告

**建议**：
1. 逐步将 `#![allow(dead_code)]` 替换为精确的 `#[allow(dead_code)]` 标注到具体项
2. 对于确实未使用的代码，评估是否应该删除
3. 优先处理 `session_core/mod.rs`——它是规则核心，不应该有大量死代码

---

## 问题 6: session_core 单文件过大

**严重程度**：低

**现象**：
`src/gameplay/session_core/mod.rs` 约 1200 行，包含：
- 规则数据结构定义（SessionRuleContext、RewardDraft、ShopDraft 等）
- 决策函数（on_room_enter、on_room_cleared、on_death 等）
- 商店逻辑（generate_shop、apply_shop_purchase 等）
- 祝福/铭文逻辑（generate_blessing_offers 等）
- 全部单元测试（约 400 行）

**建议**：
拆分为多个文件：
```
session_core/
├── mod.rs        → 重导出 + SessionRuleContext
├── types.rs      → 数据结构定义
├── rules.rs      → 房间/死亡/奖励决策函数
├── shop.rs       → 商店生成和购买逻辑
└── tests.rs      → 单元测试
```

---

## 执行优先级

| 优先级 | 问题 | 理由 |
|--------|------|------|
| P0 | 铭文残留清理 | 设计已移除但代码完全保留，是最大的代码/设计不一致 |
| P1 | 插件注册位置 | 简单移动，立即改善架构一致性 |
| P1 | EventRoom UI 泄漏 | 同上 |
| P2 | TeamMarker 重复 | 小改动，顺手修 |
| P2 | dead_code 清理 | 配合铭文清理一起做 |
| P3 | session_core 拆分 | 工作量较大，可在后续迭代中进行 |
