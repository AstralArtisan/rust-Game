# 奖励系统重构 — 第一步：铭文数据模型 + 诅咒系统 + 祝福祠堂骨架

## Context

引入双轨奖励系统：属性成长（保留现有）+ 铭文系统（全新）。本步只做数据骨架和 UI 流程，不实现铭文的战斗效果。

铭文系统：4 个槽位（近战/远程/冲刺/终结技），每槽装 1 个铭文，装新的替换旧的。
祝福祠堂（原 Reward 房）：提供 2 个铭文选项，每个附带诅咒，诅咒持续 N 个房间后消除。

---

## 影响文件

### 新增文件

| 文件 | 内容 |
|------|------|
| `src/gameplay/rune/mod.rs` | 铭文模块入口，注册 RunePlugin |
| `src/gameplay/rune/data.rs` | RuneSlot, RuneTier, RuneId 枚举，RuneLoadout Component |
| `src/gameplay/curse/mod.rs` | CurseId, ActiveCurse, CurseState Component, CursePlugin |
| `assets/configs/runes.ron` | 铭文元数据配置 |
| `assets/configs/curses.ron` | 诅咒元数据配置 |

### 修改文件

| 文件 | 改动 |
|------|------|
| `src/gameplay/mod.rs` | 新增 `pub mod rune;` 和 `pub mod curse;` |
| `src/app.rs` | 注册 RunePlugin 和 CursePlugin |
| `src/gameplay/player/components.rs` | 在 Player spawn 时挂载 RuneLoadout 和 CurseState |
| `src/data/definitions.rs` | 新增 RuneConfig/RunesConfig 和 CurseConfig/CursesConfig 解析 |
| `src/gameplay/session_core/mod.rs` | 新增 `generate_blessing_choices()` 和 `RewardDraftMode::Blessing` |
| `src/gameplay/rewards/systems.rs` | 祝福祠堂触发流程（Reward 房进入时走 Blessing 模式） |
| `src/ui/reward_select.rs` | 新增祝福祠堂 UI（2 个铭文+诅咒选项 + 离开按钮） |
| `src/ui/hud.rs` | 铭文槽位显示 + 诅咒状态显示 |
| `src/gameplay/map/room.rs` | 祝福房生成规则调整 |

---

## 步骤一：铭文数据模型

### 1a. 新建 `src/gameplay/rune/data.rs`

```rust
use serde::{Deserialize, Serialize};
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuneSlot {
    Melee,
    Ranged,
    Dash,
    Finisher,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuneTier {
    Common,
    Elite,
    Legendary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuneId {
    // 近战 Common
    ImpactWave,
    SlowOnHit,
    ThirdStrikeExpand,
    // 近战 Elite
    WhirlSlash,
    ChainLightning,
    ExplosiveFist,
    VampireBlade,
    FrostTouch,
    // 远程 Common
    PierceOne,
    MarkOnHit,
    RapidFireWeak,
    // 远程 Elite
    Scatter,
    HomingBullet,
    VenomShot,
    BarrageMode,
    // 冲刺 Common
    DashEndShockwave,
    DashFirstCrit,
    Afterimage,
    // 冲刺 Elite
    ShadowClone,
    PhaseDash,
    BlinkDash,
    // 终结技 Elite
    GroundSplitter,
    BoomerangBlade,
    DeathChain,
    WeaknessExpose,
    StormField,
    InstantThunder,
    // 传说
    PhoenixSoul,
    Berserker,
    ThornBody,
    EnergyShield,
}

#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuneLoadout {
    pub melee: Option<RuneId>,
    pub ranged: Option<RuneId>,
    pub dash: Option<RuneId>,
    pub finisher: Option<RuneId>,
}

impl RuneLoadout {
    pub fn get(&self, slot: RuneSlot) -> Option<RuneId> {
        match slot {
            RuneSlot::Melee => self.melee,
            RuneSlot::Ranged => self.ranged,
            RuneSlot::Dash => self.dash,
            RuneSlot::Finisher => self.finisher,
        }
    }

    pub fn equip(&mut self, slot: RuneSlot, rune: RuneId) -> Option<RuneId> {
        let slot_ref = match slot {
            RuneSlot::Melee => &mut self.melee,
            RuneSlot::Ranged => &mut self.ranged,
            RuneSlot::Dash => &mut self.dash,
            RuneSlot::Finisher => &mut self.finisher,
        };
        let old = slot_ref.take();
        *slot_ref = Some(rune);
        old
    }
}
```

### 1b. 新建 `src/gameplay/rune/mod.rs`

```rust
pub mod data;

use bevy::prelude::*;

pub struct RunePlugin;

impl Plugin for RunePlugin {
    fn build(&self, _app: &mut App) {
        // 后续步骤注册系统
    }
}
```

### 1c. 配置文件 `assets/configs/runes.ron`

每个铭文一条记录，格式：

```ron
(
  runes: [
    ( id: ImpactWave, slot: Melee, tier: Common, title: "命中冲击波", description: "近战命中释放小范围冲击波", drawback: "", shop_cost: 120 ),
    ( id: SlowOnHit, slot: Melee, tier: Common, title: "霜击", description: "近战命中减速敌人1秒", drawback: "", shop_cost: 120 ),
    ( id: ThirdStrikeExpand, slot: Melee, tier: Common, title: "重击", description: "每第3下近战范围×1.5", drawback: "", shop_cost: 120 ),
    ( id: WhirlSlash, slot: Melee, tier: Elite, title: "回旋斩", description: "近战变360°旋转攻击", drawback: "攻速-30%", shop_cost: 200 ),
    ( id: ChainLightning, slot: Melee, tier: Elite, title: "连锁闪电", description: "命中时闪电跳到附近2敌(40%伤害)", drawback: "", shop_cost: 200 ),
    ( id: ExplosiveFist, slot: Melee, tier: Elite, title: "爆裂拳", description: "每第3次命中产生爆炸", drawback: "前两下伤害-15%", shop_cost: 200 ),
    ( id: VampireBlade, slot: Melee, tier: Elite, title: "吸血刃", description: "近战伤害8%转化为HP", drawback: "近战范围-25%", shop_cost: 200 ),
    ( id: FrostTouch, slot: Melee, tier: Elite, title: "冰霜触碰", description: "命中冻结敌人0.5秒", drawback: "攻击间隔+20%", shop_cost: 200 ),
    ( id: PierceOne, slot: Ranged, tier: Common, title: "穿透弹", description: "弹道穿透1个敌人", drawback: "", shop_cost: 120 ),
    ( id: MarkOnHit, slot: Ranged, tier: Common, title: "标记弹", description: "命中标记敌人3秒(受伤+15%)", drawback: "", shop_cost: 120 ),
    ( id: RapidFireWeak, slot: Ranged, tier: Common, title: "速射", description: "射速+30%", drawback: "伤害-15%", shop_cost: 120 ),
    ( id: Scatter, slot: Ranged, tier: Elite, title: "散射弹", description: "每次射击发射3颗弹", drawback: "每颗伤害-50%", shop_cost: 220 ),
    ( id: HomingBullet, slot: Ranged, tier: Elite, title: "追踪弹", description: "弹道轻微追踪最近敌人", drawback: "弹速-30%", shop_cost: 220 ),
    ( id: VenomShot, slot: Ranged, tier: Elite, title: "毒液弹", description: "命中附加3秒持续伤害(60%命中伤害)", drawback: "直接命中伤害-20%", shop_cost: 220 ),
    ( id: BarrageMode, slot: Ranged, tier: Elite, title: "弹幕模式", description: "射速×2", drawback: "每颗伤害-60%", shop_cost: 220 ),
    ( id: DashEndShockwave, slot: Dash, tier: Common, title: "冲击波", description: "冲刺终点产生冲击波伤害周围敌人", drawback: "", shop_cost: 120 ),
    ( id: DashFirstCrit, slot: Dash, tier: Common, title: "先手暴击", description: "冲刺后1秒内首次攻击暴击率+30%", drawback: "", shop_cost: 120 ),
    ( id: Afterimage, slot: Dash, tier: Common, title: "残影", description: "冲刺路径对经过的敌人造成伤害", drawback: "", shop_cost: 120 ),
    ( id: ShadowClone, slot: Dash, tier: Elite, title: "影分身", description: "冲刺起点留下分身2秒自动攻击", drawback: "", shop_cost: 220 ),
    ( id: PhaseDash, slot: Dash, tier: Elite, title: "相位冲刺", description: "冲刺距离×1.5全程无敌", drawback: "冷却+40%", shop_cost: 220 ),
    ( id: BlinkDash, slot: Dash, tier: Elite, title: "闪现", description: "冲刺变瞬移冷却-30%", drawback: "无无敌帧", shop_cost: 220 ),
    ( id: GroundSplitter, slot: Finisher, tier: Elite, title: "裂地斩", description: "剑气命中地面留下3秒灼烧地带", drawback: "", shop_cost: 250 ),
    ( id: BoomerangBlade, slot: Finisher, tier: Elite, title: "回旋刃", description: "剑气飞出后返回来回各一次伤害", drawback: "", shop_cost: 250 ),
    ( id: DeathChain, slot: Finisher, tier: Elite, title: "死亡连锁", description: "被标记目标死亡时标记传递给最近敌人", drawback: "", shop_cost: 250 ),
    ( id: WeaknessExpose, slot: Finisher, tier: Elite, title: "弱点暴露", description: "标记不造成即时伤害目标5秒受伤×2", drawback: "", shop_cost: 250 ),
    ( id: StormField, slot: Finisher, tier: Elite, title: "雷暴领域", description: "冲刺路径变为持续4秒电场区域", drawback: "", shop_cost: 250 ),
    ( id: InstantThunder, slot: Finisher, tier: Elite, title: "瞬雷", description: "距离变0以自身为中心释放全屏闪电", drawback: "", shop_cost: 250 ),
    ( id: PhoenixSoul, slot: Dash, tier: Legendary, title: "不死鸟", description: "每层首次致死伤害改为回复1HP+2秒无敌", drawback: "", shop_cost: 0 ),
    ( id: Berserker, slot: Melee, tier: Legendary, title: "狂战士", description: "HP<30%时攻击力+50%攻速+30%", drawback: "", shop_cost: 0 ),
    ( id: ThornBody, slot: Dash, tier: Legendary, title: "荆棘之体", description: "受伤时反弹30%伤害给攻击者", drawback: "", shop_cost: 0 ),
    ( id: EnergyShield, slot: Finisher, tier: Legendary, title: "能量护盾", description: "能量满时自动消耗50能量抵挡致命伤害", drawback: "", shop_cost: 0 ),
  ],
)
```

### 1d. `src/data/definitions.rs` 新增

在 `GameDataRegistry` 中新增 `pub runes: RunesConfig` 字段，并在加载逻辑中解析 `runes.ron`。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuneConfig {
    pub id: RuneId,
    pub slot: RuneSlot,
    pub tier: RuneTier,
    pub title: String,
    pub description: String,
    pub drawback: String,
    pub shop_cost: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunesConfig {
    pub runes: Vec<RuneConfig>,
}
```

同样新增 `CurseConfig` 和 `CursesConfig`。

---

## 步骤二：诅咒系统

### 2a. 新建 `src/gameplay/curse/mod.rs`

```rust
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CurseId {
    Fragile,     // 受伤+25%
    Sluggish,    // 移速-20%
    Exhaustion,  // 能量获取-40%
    Exposed,     // 冲刺冷却+50%
    Weakness,    // 造成伤害-20%
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveCurse {
    pub curse: CurseId,
    pub rooms_remaining: u32,
}

#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurseState {
    pub active: Vec<ActiveCurse>,
}

impl CurseState {
    pub fn has_any_curse(&self) -> bool {
        !self.active.is_empty()
    }

    pub fn add_curse(&mut self, curse: CurseId, duration: u32) {
        self.active.push(ActiveCurse { curse, rooms_remaining: duration });
    }

    /// 每次进入新房间时调用，返回刚消除的诅咒列表
    pub fn tick_room(&mut self) -> Vec<CurseId> {
        let mut expired = Vec::new();
        self.active.retain_mut(|c| {
            c.rooms_remaining = c.rooms_remaining.saturating_sub(1);
            if c.rooms_remaining == 0 {
                expired.push(c.curse);
                false
            } else {
                true
            }
        });
        expired
    }

    pub fn damage_taken_mult(&self) -> f32 {
        let mut mult = 1.0;
        for c in &self.active {
            if c.curse == CurseId::Fragile { mult *= 1.25; }
        }
        mult
    }

    pub fn move_speed_mult(&self) -> f32 {
        let mut mult = 1.0;
        for c in &self.active {
            if c.curse == CurseId::Sluggish { mult *= 0.80; }
        }
        mult
    }

    pub fn energy_gain_mult(&self) -> f32 {
        let mut mult = 1.0;
        for c in &self.active {
            if c.curse == CurseId::Exhaustion { mult *= 0.60; }
        }
        mult
    }

    pub fn dash_cooldown_mult(&self) -> f32 {
        let mut mult = 1.0;
        for c in &self.active {
            if c.curse == CurseId::Exposed { mult *= 1.50; }
        }
        mult
    }

    pub fn damage_dealt_mult(&self) -> f32 {
        let mut mult = 1.0;
        for c in &self.active {
            if c.curse == CurseId::Weakness { mult *= 0.80; }
        }
        mult
    }
}

pub struct CursePlugin;

impl Plugin for CursePlugin {
    fn build(&self, _app: &mut App) {
        // 后续注册诅咒递减系统
    }
}
```

### 2b. 配置文件 `assets/configs/curses.ron`

```ron
(
  curses: [
    ( id: Fragile, title: "脆弱", description: "受到伤害+25%", duration: 3 ),
    ( id: Sluggish, title: "迟缓", description: "移速-20%", duration: 3 ),
    ( id: Exhaustion, title: "枯竭", description: "能量获取-40%", duration: 3 ),
    ( id: Exposed, title: "暴露", description: "冲刺冷却+50%", duration: 2 ),
    ( id: Weakness, title: "虚弱", description: "造成伤害-20%", duration: 3 ),
  ],
)
```

---

## 步骤三：Player 挂载新 Component

在 `src/gameplay/player/components.rs` 或 Player spawn 逻辑中，给 Player entity 新增：

```rust
RuneLoadout::default(),
CurseState::default(),
```

找到 Player spawn 的位置（搜索 `commands.spawn` 附近有 `Health`、`RewardModifiers` 的地方），在 bundle 中追加这两个 Component。

---

## 步骤四：祝福祠堂流程

### 4a. `session_core/mod.rs` 新增

在 `RewardDraftMode` 枚举中新增 `Blessing` 变体：

```rust
pub enum RewardDraftMode {
    SingleBuff,
    HealOrBuff,
    DualBuff,
    LoneSurvivor,
    Blessing,  // 新增：祝福祠堂模式
}
```

新增函数 `generate_blessing_choices`：

```rust
pub fn generate_blessing_choices(
    rng: &mut GameRng,
    floor_number: u32,
    rune_loadout: &RuneLoadout,
    runes_config: &RunesConfig,
    curses_config: &CursesConfig,
) -> Vec<BlessingOffer> {
    // 1. 根据楼层决定等级池：Floor2-3 精英为主，Floor4 必含传说
    // 2. 过滤掉玩家已装备的同 ID 铭文
    // 3. 随机选 2 个铭文
    // 4. 每个铭文随机配一个诅咒
    // 返回 Vec<BlessingOffer>，长度为 2
}

pub struct BlessingOffer {
    pub rune_id: RuneId,
    pub rune_slot: RuneSlot,
    pub rune_tier: RuneTier,
    pub rune_title: String,
    pub rune_description: String,
    pub rune_drawback: String,
    pub curse_id: CurseId,
    pub curse_title: String,
    pub curse_description: String,
    pub curse_duration: u32,
}
```

### 4b. Reward 房进入逻辑修改

在 `on_room_enter` 中，当 `room_type == RoomType::Reward` 时：
- 检查 `CurseState::has_any_curse()`：如果有诅咒，不触发祝福（跳过，房间为空）
- 检查楼层：Floor 1 不触发
- 否则返回 `RewardDraftMode::Blessing`

```rust
RoomType::Reward => {
    if floor_number <= 1 || has_active_curse {
        RoomEnterDecision { reward_mode: None, auto_open_shop: false }
    } else {
        RoomEnterDecision { reward_mode: Some(RewardDraftMode::Blessing), auto_open_shop: false }
    }
}
```

### 4c. `rewards/systems.rs` 处理 Blessing 模式

当 `RewardDraftMode::Blessing` 时：
1. 调用 `generate_blessing_choices` 生成 2 个 BlessingOffer
2. 存入新 Resource `BlessingFlow`
3. 切换到 `AppState::RewardSelect`（复用现有状态，UI 根据模式切换显示）

---

## 步骤五：祝福祠堂 UI

### 5a. `ui/reward_select.rs` 新增 Blessing 模式

在 `spawn_reward_select_ui` 中，检测 `RewardFlowMode::Blessing`（新增变体）时：

布局：
```
┌─────────────────────────────────────┐
│         祝福祠堂                      │
├────────────────┬────────────────────┤
│  [1] 铭文A      │  [2] 铭文B          │
│  名称 + 描述    │  名称 + 描述        │
│  取舍（红字）    │  取舍（红字）        │
│  ─────────     │  ─────────         │
│  诅咒：脆弱     │  诅咒：迟缓          │
│  受伤+25%      │  移速-20%           │
│  持续3房间      │  持续3房间           │
├────────────────┴────────────────────┤
│           [Esc] 离开                  │
└─────────────────────────────────────┘
```

- 按 1 选择左侧：装备铭文A + 获得诅咒A
- 按 2 选择右侧：装备铭文B + 获得诅咒B
- 按 Esc 离开：不拿任何东西

选择后：
1. `RuneLoadout::equip(slot, rune_id)` 装备铭文
2. `CurseState::add_curse(curse_id, duration)` 添加诅咒
3. 退出 RewardSelect 状态

---

## 步骤六：HUD 显示

### 6a. 铭文槽位（`ui/hud.rs`）

在技能栏上方或旁边显示 4 个小方块：
- 空槽：灰色边框 `Color::srgb(0.3, 0.3, 0.3)`
- 已装备：根据槽位着色（近战红、远程蓝、冲刺绿、终结技金）+ 铭文名称首字

### 6b. 诅咒状态

在 HP 条下方显示当前诅咒：
- 红色文字：诅咒名称 + 剩余房间数
- 例如："脆弱 (3)" 显示为红色小字

---

## 步骤七：祝福房生成规则

在房间生成逻辑中（`rooms.ron` 或 `session_core` 中的房间布局函数）：
- 每层最多 1 个 Reward 房
- Floor 1 不生成 Reward 房
- 如果当前有诅咒，跳过 Reward 房生成（或生成但进入时为空）

找到房间布局生成的代码（搜索 `RoomType::Reward` 的分配逻辑），添加这些限制。

---

## 步骤八：注册到 App

在 `src/app.rs` 的 `GamePlugin::build` 中：

```rust
.add_plugins(gameplay::rune::RunePlugin)
.add_plugins(gameplay::curse::CursePlugin)
```

在 `src/gameplay/mod.rs` 中：

```rust
pub mod rune;
pub mod curse;
```

---

## 验证方法

```bash
cargo check --quiet
cargo test --quiet   # 期望：33+ passed
```

**此步不实现铭文战斗效果**。验证目标：
1. 编译通过
2. 测试通过
3. Reward 房在 Floor 2+ 进入时显示祝福祠堂 UI（2 个铭文+诅咒选项）
4. 选择后 HUD 显示铭文槽位和诅咒倒计时
5. 诅咒在经过 N 个房间后自动消除
6. 有诅咒时不再出现新的祝福房
