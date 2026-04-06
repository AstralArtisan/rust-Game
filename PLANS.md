# 奖励系统重构 + 怪物扩充

## Context

试玩反馈四个核心问题：
1. 奖励房太少（4 层只见到 1 个），角色成长存在感低
2. 所有成长走同一管道（三选一），频率低、形式单一、缺乏构建感
3. 小怪池太小，精英与普通怪无机制区别
4. TideHunter（Floor 3 Boss）威胁太低，太好打

重构目标：将成长拆为两层独立系统，大幅增加获取频率和构建多样性，同时扩充怪物池和精英机制。

---

## 设计总览

### 两层成长体系

**第 1 层：属性成长（确定性）**
- 经验值 + 升级制：击杀敌人获得 XP，累积升级
- 升级时从 2-3 个属性选项中选 1 个（+ATK, +HP, +Speed, +Crit 等）
- 商店仍可购买属性强化
- Boss 击杀自动给大幅属性提升

**第 2 层：强化构建（随机性）**
- 统一的"强化"(Augment) 系统，替代旧的铭文+精通
- ~30 个强化，分 3 个稀有度（普通/精英/传说）
- 4 个类别：近战、远程、机动、通用
- 同类强化可升级（第 2 次获得 → 强化版）
- 无槽位限制，实际上限约 8 个/局
- 多渠道获取：战斗掉落、精英房、Boss、商店、事件房、祝福祠堂

### 强化池

**近战类（8 个）**
| ID | 稀有度 | 效果 | 升级效果 |
|----|--------|------|----------|
| LifestealSlash | 普通 | 近战命中回复 3% 伤害为 HP | 5% |
| HeavyStrike | 普通 | 近战击退 +80%，伤害 +15% | 击退 +120%，伤害 +25% |
| ComboAccelerate | 普通 | 连击 5+ 时攻速 +25% | 连击 3+ 时攻速 +40% |
| Whirlwind | 精英 | 近战攻击变为 360° 旋风（伤害 70%） | 旋风伤害 100% |
| ArmorBreak | 精英 | 近战命中降低敌人受伤抗性 20%，持续 3s | 30%，5s |
| Reflect | 精英 | 近战攻击反弹附近弹幕 | 反弹弹幕伤害 +50% |
| SwordWave | 传说 | 近战释放远程剑气（35% 伤害） | 剑气穿透 + 50% 伤害 |
| Executioner | 传说 | 敌人 HP<15% 时近战秒杀 | HP<25% |

**远程类（8 个）**
| ID | 稀有度 | 效果 | 升级效果 |
|----|--------|------|----------|
| Piercing | 普通 | 弹丸穿透 1 个敌人 | 穿透 2 个 |
| SpeedBoost | 普通 | 弹速 +30% | +50% |
| ExtraProjectile | 普通 | 每次射击额外 +1 弹 | +2 弹 |
| Homing | 精英 | 弹丸轻微追踪最近敌人 | 强追踪 |
| ChainLightning | 精英 | 命中后闪电跳到 1 个附近敌人（50% 伤害） | 跳 2 个 |
| Scatter | 精英 | 射击变为 3 发扇形（每发 50% 伤害） | 5 发扇形 |
| BulletStorm | 传说 | 终结技改为全屏弹幕（8 方向 ×3 波） | 12 方向 ×5 波 |
| Freeze | 传说 | 远程命中 15% 概率冻结敌人 1.5s | 25% 概率，2s |

**机动类（6 个）**
| ID | 稀有度 | 效果 | 升级效果 |
|----|--------|------|----------|
| DashTrail | 普通 | 冲刺留下伤害轨迹（ATK×40%） | ATK×70% |
| DashEnergy | 普通 | 冲刺穿过敌人回复 10 能量 | 15 能量 |
| ExtendedInvuln | 普通 | 冲刺无敌时间 +0.15s | +0.25s |
| DashReset | 精英 | 击杀敌人刷新冲刺冷却 | 击杀 +30% 移速 2s |
| DashShield | 精英 | 冲刺结束获得护盾（吸收 1 次伤害，3s） | 护盾持续 5s |
| Blink | 传说 | 冲刺改为瞬移（无中间帧） | 瞬移距离 +50% |

**通用类（8 个）**
| ID | 稀有度 | 效果 | 升级效果 |
|----|--------|------|----------|
| GoldBonus | 普通 | 金币掉落 +25% | +50% |
| XpBonus | 普通 | 经验获取 +25% | +50% |
| PickupRange | 普通 | 拾取范围 +60% | +100% |
| Thorns | 精英 | 受伤时反弹 15 点伤害 | 25 点 |
| KillHeal | 精英 | 击杀回复 5 HP | 8 HP |
| CritEnhance | 精英 | 暴击率 +10%，暴击伤害 +30% | +15%，+50% |
| Phoenix | 传说 | 死亡时复活（50% HP，每局 1 次） | 复活 80% HP |
| Greed | 传说 | 每 100 金币 → +5% 伤害 | 每 80 金币 |

### 事件房（8 种随机事件）

**风险回报型**
1. 赌博机：花 50 金币，随机获得 1 个强化（60% 普通，30% 精英，10% 传说）
2. 诅咒祭坛：接受 1 个诅咒，立即获得 1 个精英强化选择
3. 血之契约：消耗 30% 当前 HP，获得 1 个强化选择（2 选 1）

**纯收益型**
4. 宝箱房：直接获得 1 个普通强化 + 30 金币
5. 治疗泉：回复 40% 最大 HP
6. 旅商：提供 2 个半价强化可购买

**挑战型**
7. 限时挑战：30 秒内击杀一波敌人，成功获得精英强化
8. 精英遭遇：单挑 1 个带词缀精英，击杀必掉精英强化

### 商店扩展

| 区域 | 内容 | 备注 |
|------|------|------|
| 属性区 | HP/ATK/速度/暴击/攻速 | 保留现有 |
| 强化区 | 2-3 个随机强化 | 新增，价格按稀有度 |
| 消耗品 | 回血药水、临时增益 | 新增 |
| 诅咒移除 | 花 80 金币移除当前诅咒 | 新增 |

### 新增小怪（3 种）

| 类型 | 解锁层 | 机制 |
|------|--------|------|
| Bomber | Floor 2+ | 靠近后蓄力 1s 自爆，范围伤害。蓄力期间可击杀阻止 |
| Shielder | Floor 3+ | 正面免疫远程伤害，缓慢推进保护后排。需绕背或近战 |
| Summoner | Floor 4+ | 远离玩家，周期召唤 1-2 个小型 MeleeChaser。本体脆弱 |

### 精英词缀系统

精英怪 = 普通怪 + 1 个随机词缀（替代纯数值放大）

| 词缀 | 效果 |
|------|------|
| Swift | 移速 +50%，攻速 +30% |
| Splitting | 死亡时分裂为 2 个弱化版本（50% HP，70% 伤害） |
| Shielded | 战斗开始有 1 层护盾吸收 1 次伤害 |
| Vampiric | 命中玩家回复自身 10% 最大 HP |
| Berserk | HP<30% 时伤害翻倍，变红 |
| Teleporting | 每 3s 短距离闪现 |

视觉：体型 1.3x，金色光环，词缀图标显示在头顶。

### TideHunter 数值调整

| 参数 | 旧值 | 新值 |
|------|------|------|
| Stalk 时间 (P1/P2/P3) | 1.8/1.4/1.0 | 1.2/0.8/0.5 |
| 影子伤害倍率 | contact_damage × 0.6 | × 1.0 |
| P3 影子持续 | 4.5s | 6.0s |
| 穿越时直接伤害 | 无 | 穿越路径上的玩家受 contact_damage |
| P2+ 目标预判 | 无 | 预判玩家移动方向 |

---

## 实施阶段

### 阶段 1：强化数据模型 + XP/升级系统（基础骨架）✅ 已完成

**目标**：替换旧铭文数据模型为强化系统，加入 XP/升级，游戏可编译可运行。

## 实施阶段

### 阶段 1：强化数据模型 + XP/升级系统 ✅ 已完成
### 阶段 2：强化选择 UI + 升级选择 UI ✅ 已完成
### 阶段 3：30 个强化战斗效果 ✅ 已完成

## Current Task

### 阶段 4a：掉落物系统 (Drop/Loot)

**目标**：敌人死亡后掉落物理实体（金币/XP球），玩家靠近自动拾取，PickupRange 强化生效。

**Affected files:**
- `src/gameplay/drops/mod.rs` (新建)
- `src/gameplay/mod.rs` (修改)
- `src/gameplay/enemy/systems.rs` (修改)

#### 1. 新建 `src/gameplay/drops/mod.rs`

创建 `DropPlugin` 和掉落物系统。

**组件：**
```rust
#[derive(Component)]
pub struct DroppedItem {
    pub kind: DropKind,
    pub value: u32,
    pub lifetime: Timer,  // 15s 后 despawn
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropKind { Gold, Xp }

#[derive(Component)]
pub struct DropVelocity(pub Vec2);

#[derive(Component)]
pub struct DropBob(pub f32);
```

**Plugin 注册 5 个系统**（全部 `Update`, run_if `in_state(AppState::InGame).or_else(in_state(AppState::CoopGame).and_then(is_coop_authority).and_then(is_coop_simulation_active))`）：

1. `spawn_drops_on_death` — 读 `DeathEvent`（`event.team == Team::Enemy`），查询死亡 entity 的 `GlobalTransform` + `EnemyKind` + `Option<EliteMarker>`。在死亡位置生成金币+XP 实体。
   - 金币数值：Boss = 30/45/58/70 by floor, 其他 = 8/10/13/16 by floor, elite +`data.balance.elite_gold_bonus`
   - XP 数值：Boss = 100+(floor-1)*30 cap 200, elite = 25+(floor-1)*5 cap 40, normal = 8+(floor-1)*2 cap 15
   - GoldBonus 乘数：查询所有 Player 的 AugmentInventory，stacks(GoldBonus) 2→1.50, 1→1.25, 0→1.0。每个 player 生成独立的金币 drop（乘数不同）
   - XpBonus 乘数：同理，stacks(XpBonus) 2→1.50, 1→1.25
   - 每个 drop：随机散射方向（`rng.gen_range_f32(0, TAU)` 得到角度），速度 120-180 随机
   - Sprite: 金币 8×8 `Color::srgb(0.95, 0.85, 0.25)`, XP 6×6 `Color::srgb(0.35, 0.85, 0.95)`, 用 `assets.textures.white`
   - 标记 `InGameEntity` + `Name::new("GoldDrop"/"XpDrop")`
   - lifetime: `Timer::from_seconds(15.0, TimerMode::Once)`
   - **排序**：`.before(crate::gameplay::enemy::systems::enemy_death_system)`

2. `drop_physics` — 对每个 `(DropVelocity, DropBob, Transform)`：
   - `vel.0 *= (-6.0 * dt).exp()` 指数衰减
   - `bob.0 += dt * 4.0`（相位递增）
   - `tf.translation.x += vel.0.x * dt`
   - `tf.translation.y += vel.0.y * dt + (bob.0.sin() * 3.0 * dt)` （bob 效果）

3. `drop_magnet` — 对每个 `(DroppedItem, DropVelocity, GlobalTransform)`，查询所有 `(Player, GlobalTransform, Option<AugmentInventory>)`：
   - pickup_mult = match stacks(PickupRange) { 2 => 2.0, 1 => 1.6, _ => 1.0 }
   - magnet_range = 140.0 * pickup_mult
   - 找最近 player，如果距离 < magnet_range：
     - dir = (player_pos - drop_pos).normalize_or_zero()
     - vel.0 += dir * 600.0 * dt（加速朝玩家）

4. `drop_collect` — 对每个 `(Entity, DroppedItem, GlobalTransform)`，查询所有 `(Player, GlobalTransform, &mut Gold, Option<AugmentInventory>)`：
   - 找最近 player，距离 < 28.0 时：
     - Gold: `gold.0 += item.value`
     - Xp: 发送 `XpGainEvent { amount: item.value }`
     - 发送 `SfxEvent { kind: SfxKind::RewardPickup }`
     - `safe_despawn_recursive(commands, entity)`

5. `drop_expire` — 对每个 `(Entity, &mut DroppedItem)`：
   - `item.lifetime.tick(time.delta())`
   - if finished: `safe_despawn_recursive(commands, entity)`

**系统顺序**：`spawn_drops_on_death.before(enemy_death_system)`, 其余默认顺序

#### 2. 修改 `src/gameplay/mod.rs`

在 module 声明中添加 `pub mod drops;`，在 `GameplayPlugin::build()` 的 `add_plugins()` 中添加 `drops::DropPlugin`。

#### 3. 修改 `src/gameplay/enemy/systems.rs`

在 `enemy_death_system` 中：
- **删除** ~L962 的 `xp_events.send(XpGainEvent { amount: xp_amount });`
- **删除** ~L964-976 的 player 循环中 gold 计算和 `gold.0 = gold.0.saturating_add(final_gold)` 部分
- **保留** 同一循环中的 kill_heal、dash_reset、charge_gain、lifesteal 逻辑
- 注意：删除 gold 相关代码后，如果 `gold` 变量不再使用，需要从 query 的解构中移除或加 `_` 前缀
- 同时删除 base_gold、reward_gold、xp_amount 的计算代码（已搬到 drops 模块）

#### 验证命令
```bash
cargo check --quiet
cargo test --quiet
```

---

## 验证方法

每个阶段完成后：
```bash
cargo check --quiet
cargo test --quiet
```

最终验证：手动游玩完整 4 层，检查掉落物、商店、事件房功能。
