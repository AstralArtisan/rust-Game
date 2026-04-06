# Phase 4b: 商店扩展

## Context

Phase 4a（掉落物系统）已完成。Phase 4b 扩展商店：新增强化购买区、消耗品区（回血药水、诅咒移除）。

当前商店只卖 8 种属性（治疗/强健/锋刃/迅捷/轻盈/充能/锐眼/连击），按 1/2/3 购买 3 个随机属性。需要扩展为三个区域。

## Current Task

### 目标

商店从单一属性区扩展为三区：属性区（保留）+ 强化区（新增）+ 工具区（新增）。

### Affected files

- `src/gameplay/shop/mod.rs` (修改)
- `src/gameplay/session_core/mod.rs` (修改)
- `src/ui/shop.rs` (修改)

### 详细改动

#### 1. 修改 `src/gameplay/session_core/mod.rs`

**扩展 `SharedShopItem` 枚举**（~L131）：
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedShopItem {
    // 现有 8 个属性项不变
    Heal,
    IncreaseMaxHealth,
    IncreaseAttackPower,
    ReduceDashCooldown,
    IncreaseMoveSpeed,
    IncreaseEnergyMax,
    IncreaseCritChance,
    IncreaseAttackSpeed,
    // 新增
    Augment(AugmentId),
    HealingPotion,    // 回复 25% max HP, 30g
    RemoveCurse,      // 移除最旧诅咒, 80g
}
```

注意：`SharedShopItem` 当前 derive 了 `Copy`。`AugmentId` 也是 Copy，所以 `Augment(AugmentId)` 仍然是 Copy。

**扩展 `ShopDraft`**（~L148）：
```rust
pub struct ShopDraft {
    pub refresh_count: u32,
    pub offers: Vec<ShopOfferDraft>,        // 属性区（保留）
    pub augment_offers: Vec<ShopOfferDraft>, // 强化区（新增）
    pub utility_offers: Vec<ShopOfferDraft>, // 工具区（新增）
}
```

**修改 `build_shop_offers()`**（~L485）：保持现有属性区逻辑不变。

**新增 `build_augment_offers()` 函数**：
- 参数：`registry: &GameDataRegistry, rng: &mut GameRng, floor_number: u32`
- 从 `registry.augments.augments` 中随机选 2-3 个（floor 1-2 选 2 个，floor 3+ 选 3 个）
- 价格用 augment 定义的 `shop_cost` 字段（如果有），否则按稀有度：Common=40, Elite=70, Legendary=120
- 返回 `Vec<ShopOfferDraft>`，item 为 `SharedShopItem::Augment(augment_id)`

**新增 `build_utility_offers()` 函数**：
- 始终包含 `HealingPotion`（cost=30）
- 如果玩家有诅咒（需要传入 `has_curse: bool` 参数），包含 `RemoveCurse`（cost=80）
- 返回 `Vec<ShopOfferDraft>`

**修改 `build_shop_draft()`**：调用三个 build 函数填充三个区域。新增参数 `registry: Option<&GameDataRegistry>` 和 `has_curse: bool`。

**修改 `refresh_shop_draft()`**：同样刷新三个区域。新增同样参数。

**扩展 `apply_shop_item()`**（~L516）：新增 match 分支：
```rust
SharedShopItem::Augment(_) => false, // 强化购买不在这里处理，返回 false
SharedShopItem::HealingPotion => {
    effects.health.current = (effects.health.current + effects.health.max * 0.25).min(effects.health.max);
    true
}
SharedShopItem::RemoveCurse => false, // 诅咒移除不在这里处理，返回 false
```

#### 2. 修改 `src/gameplay/shop/mod.rs`

**扩展 `ShopItem` 枚举**（~L106）：
```rust
#[derive(Debug, Clone, Copy)]
pub enum ShopItem {
    // 现有 8 个不变
    Heal, IncreaseMaxHealth, IncreaseAttackPower, ReduceDashCooldown,
    IncreaseMoveSpeed, IncreaseEnergyMax, IncreaseCritChance, IncreaseAttackSpeed,
    // 新增
    Augment(AugmentId),
    HealingPotion,
    RemoveCurse,
}
```

**扩展 `ShopOffers`**（~L53）：
```rust
pub struct ShopOffers {
    pub room: Option<RoomId>,
    pub lines: Vec<ShopLine>,              // 属性区
    pub augment_lines: Vec<ShopLine>,      // 强化区
    pub utility_lines: Vec<ShopLine>,      // 工具区
    pub refresh_count: u32,
}
```

**扩展 `CachedShopState`**（~L70）：同样新增 `augment_lines` 和 `utility_lines`。

**修改 `generate_shop_offers()`**：从 draft 的三个区域分别构建 lines。需要新增参数传入 `GameDataRegistry` 和 `has_curse`。

**修改 `refresh_shop_offers()`**：同上。

**修改 `build_shop_lines_from_draft()`**：拆分为三个函数或一个函数处理三个区域。对 `Augment(id)` 类型，从 registry 获取名称和描述。

**扩展 `shop_item_from_shared()` 和 `shared_shop_item_from_shop_item()`**：新增三个变体的映射。

**扩展 `describe_item()` 和 `describe_item_local()`**：
```rust
ShopItem::Augment(_) => ("强化", "获得一个强化", base_cost),
ShopItem::HealingPotion => ("回血药水", "回复 25% 最大生命", 30),
ShopItem::RemoveCurse => ("净化", "移除一个诅咒", 80),
```

**修改 `handle_shop_purchase_input()`**（~L365）：
- 按键映射：1/2/3 = 属性区，4/5/6 = 强化区，7/8 = 工具区
- 新增 `AugmentInventory` 和 `CurseState` 到 player query（用 `Option<>`）
- Augment 购买：`inventory.add(augment_id)`，不走 `apply_shop_purchase`
- RemoveCurse 购买：`curse_state.active.remove(0)`（移除最旧的），不走 `apply_shop_purchase`
- HealingPotion：走 `apply_shop_purchase` 即可
- 购买后更新对应区域的 `purchased` 标记和 cache

**修改 `maybe_enter_shop_state()` 和 `open_shop_hotkey()`**：传入新参数（registry, has_curse）给 `generate_shop_offers`。`has_curse` 从 player 的 `CurseState` 查询。

#### 3. 修改 `src/ui/shop.rs`

**修改 `setup_shop_ui()`**：更新说明文字为 "1/2/3 属性 | 4/5/6 强化 | 7/8 工具 | R 刷新 | Esc 关闭"

**修改 `update_shop_ui()`**：分三区渲染，每区有标题（"属性"/"强化"/"工具"）。遍历 `offers.lines`、`offers.augment_lines`、`offers.utility_lines`，按键编号分别从 1/4/7 开始。

### 需要注意的导入

- `src/gameplay/shop/mod.rs` 需要新增导入：
  - `use crate::gameplay::augment::data::{AugmentId, AugmentInventory};`
  - `use crate::gameplay::curse::CurseState;`
- `src/gameplay/session_core/mod.rs` 需要新增导入：
  - `use crate::gameplay::augment::data::AugmentId;`（如果还没有的话）

### 验证命令
```bash
cargo check --quiet
cargo test --quiet
```
