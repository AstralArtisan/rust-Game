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

## Current Task

### 阶段 3a：普通(Common)强化战斗效果实现（12 个）

**目标**：实现全部 12 个 Common 稀有度强化的战斗效果。

**实现策略**：
- 在现有战斗系统函数中添加 `AugmentInventory` 查询
- 用 `inventory.has(AugmentId::X)` 和 `inventory.stacks(AugmentId::X)` 检查效果
- stacks==1 为基础效果，stacks==2 为升级效果
- 新增 `src/gameplay/augment/effects.rs` 存放新系统

---

#### 1. LifestealSlash — 近战吸血
- **效果**：近战命中回复 3% 伤害为 HP（升级: 5%）
- **文件**：`src/gameplay/combat/hitbox.rs` → `detect_hitbox_hurtbox_overlap`
- **实现**：在现有 melee lifesteal 逻辑附近（约 Line 178-224），增加 AugmentInventory 检查：
  ```
  // 在 DamageKind::PlayerMelee 命中后，查找 hitbox.owner 对应的 player
  // if player has AugmentId::LifestealSlash:
  //   fraction = if stacks >= 2 { 0.05 } else { 0.03 }
  //   heal = (amount * fraction).min(5.0)  // cap per hit
  //   health.current = (health.current + heal).min(health.max)
  ```
- **注意**：与现有 `lifesteal_on_kill`（击杀时回血）不冲突，这是命中时回血

#### 2. HeavyStrike — 重击
- **效果**：近战击退 +80%（升级: +120%），伤害 +15%（升级: +25%）
- **文件**：`src/gameplay/player/combat.rs` → `spawn_player_melee_hitbox_with_mods`
- **实现**：在构建 Hitbox 组件前，读取 AugmentInventory：
  ```
  // 添加 AugmentInventory 参数（通过 player_attack_input_system 传入）
  // if has HeavyStrike:
  //   knockback *= if stacks >= 2 { 2.20 } else { 1.80 }
  //   damage *= if stacks >= 2 { 1.25 } else { 1.15 }
  ```

#### 3. ComboAccelerate — 连击加速
- **效果**：连击 5+ 时攻速 +25%（升级: 连击 3+ 时 +40%）
- **文件**：`src/gameplay/player/combat.rs` → `player_attack_input_system`
- **实现**：在 `cd.apply_speed_bonus()` 调用处，额外叠加连击加速：
  ```
  // 需要在 query 中添加 &Combo 和 &AugmentInventory
  // let combo_threshold = if stacks >= 2 { 3 } else { 5 };
  // let combo_bonus = if stacks >= 2 { 0.40 } else { 0.25 };
  // if combo.count >= combo_threshold:
  //   cd.apply_speed_bonus(mods.total_melee_speed_bonus() + combo_bonus)
  // else:
  //   cd.apply_speed_bonus(mods.total_melee_speed_bonus())
  ```

#### 4. SpeedBoost — 弹速提升
- **效果**：弹丸速度 +30%（升级: +50%）
- **文件**：`src/gameplay/player/combat.rs` → `spawn_ranged_projectile` 或 `player_ranged_input_system`
- **实现**：在计算 projectile_speed 时：
  ```
  // base projectile_speed = 720.0 * mods.ranged_projectile_speed_mult()
  // if has SpeedBoost:
  //   projectile_speed *= if stacks >= 2 { 1.50 } else { 1.30 }
  ```

#### 5. ExtraProjectile — 额外弹丸
- **效果**：每次射击 +1 弹（升级: +2）
- **文件**：`src/gameplay/player/combat.rs` → `spawn_player_ranged_volley` 或 `spawn_ranged_burst`
- **实现**：在弹幕生成时，添加额外弹丸（小角度偏移）：
  ```
  // 在 spawn_ranged_burst 中，读取 augment inventory
  // extra = if stacks >= 2 { 2 } else { 1 }
  // 为每个 extra 弹丸：以 ±0.15 弧度偏移生成，伤害 = 原伤害 * 0.6
  ```

#### 6. Piercing — 穿透
- **效果**：弹丸穿透 1 个敌人（升级: 2 个）
- **文件**：`src/gameplay/combat/hitbox.rs` → `detect_hitbox_hurtbox_overlap`
- **新增组件**：`PierceCount { remaining: u8 }` 在 `src/gameplay/combat/projectiles.rs`
- **实现**：
  ```
  // 1. 在 spawn_player_projectile 中：如果玩家有 Piercing 强化，挂载 PierceCount 组件
  //    remaining = if stacks >= 2 { 2 } else { 1 }
  // 2. 在 detect_hitbox_hurtbox_overlap 中：如果 hitbox entity 有 PierceCount 且 remaining > 0，
  //    不 despawn hitbox，而是 remaining -= 1，并将当前 target 加入 hit_set 避免重复命中
  // 3. 需要新增 HitTargets { set: HashSet<Entity> } 组件跟踪已命中目标
  ```

#### 7. DashTrail — 冲刺轨迹伤害
- **效果**：冲刺留下伤害轨迹（ATK×40%，升级: 70%）
- **文件**：`src/gameplay/player/dash.rs` → `update_dash_state`
- **实现**：在 trail spawning 逻辑处，将现有 `mods.dash_damage_trail` 条件扩展：
  ```
  // let has_trail = mods.dash_damage_trail || inventory.has(AugmentId::DashTrail);
  // if has_trail:
  //   let trail_mult = if inventory.stacks(DashTrail) >= 2 { 0.70 }
  //                    else if inventory.has(DashTrail) { 0.40 }
  //                    else { 0.45 };  // 原始 RewardModifiers 的值
  //   spawn_dash_trail_hitbox with damage = attack_power * trail_mult
  ```

#### 8. DashEnergy — 冲刺穿敌回能
- **效果**：冲刺穿过敌人回复 10 能量（升级: 15）
- **文件**：`src/gameplay/augment/effects.rs`（新建系统）
- **实现**：新增系统 `dash_energy_system`
  ```
  // 在 Update 中运行，run_if(in_state(InGame))
  // 检测玩家正在冲刺（DashState.active）且有 DashEnergy 强化
  // 用 spatial query 检测冲刺路径上的敌人碰撞
  // 简化实现：如果冲刺中 frame 与任意敌人 AABB 重叠，回复能量（每个敌人每次冲刺只触发一次）
  // 需要临时 Resource 或 Component 记录本次冲刺已触发的敌人
  ```
  - 简化方案：在 `update_dash_state` 中，当 trail hitbox 命中敌人时回复能量
  - 更简化：每次冲刺结束时，如果冲刺期间有 trail hitbox 命中，回复一次能量

#### 9. ExtendedInvuln — 延长无敌
- **效果**：冲刺无敌时间 +0.15s（升级: +0.25s）
- **文件**：`src/gameplay/player/dash.rs` → `player_dash_input_system`
- **实现**：在设置 invincibility timer 时：
  ```
  // let extra_invuln = match inventory.stacks(ExtendedInvuln) {
  //     2 => 0.25,
  //     1 => 0.15,
  //     _ => 0.0,
  // };
  // inv.timer = Timer::from_seconds(dash.base_duration_s + extra_invuln, TimerMode::Once);
  ```

#### 10. GoldBonus — 金币加成
- **效果**：金币掉落 +25%（升级: +50%）
- **文件**：`src/gameplay/enemy/systems.rs` → `enemy_death_system`（约 Line 918-953）
- **实现**：在计算 reward_gold 后，应用加成：
  ```
  // 在 player_q 循环内，检查 AugmentInventory
  // let gold_mult = match inventory.stacks(GoldBonus) {
  //     2 => 1.50,
  //     1 => 1.25,
  //     _ => 1.0,
  // };
  // let final_gold = (reward_gold as f32 * gold_mult) as u32;
  // gold.0 = gold.0.saturating_add(final_gold);
  ```

#### 11. XpBonus — 经验加成
- **效果**：经验获取 +25%（升级: +50%）
- **文件**：`src/gameplay/progression/experience.rs` → `process_xp_gains`
- **实现**：在应用 XP 前，查询 AugmentInventory 并乘以加成：
  ```
  // let xp_mult = match inventory.stacks(XpBonus) {
  //     2 => 1.50,
  //     1 => 1.25,
  //     _ => 1.0,
  // };
  // let adjusted_xp = (total_xp as f32 * xp_mult) as u32;
  // let levels_gained = level.add_xp(adjusted_xp);
  ```

#### 12. PickupRange — 拾取范围
- **效果**：拾取范围 +60%（升级: +100%）
- **实现**：当前游戏没有独立的拾取物系统（金币直接加到玩家身上）
- **简化处理**：暂时标记为 "被动效果 - 等阶段 4 实现掉落物系统后生效"
- 或者：在 enemy_death_system 中，增加拾取范围内的敌人也给金币/XP（类似磁铁效果）
- **决定**：暂时只实现数据模型，效果在阶段 4 掉落物系统中实现

---

**影响文件**：
| 文件 | 操作 |
|------|------|
| `src/gameplay/augment/effects.rs` | 新建 — DashEnergy 系统 |
| `src/gameplay/augment/mod.rs` | 修改 — 注册 effects 模块和系统 |
| `src/gameplay/player/combat.rs` | 修改 — HeavyStrike, ComboAccelerate, SpeedBoost, ExtraProjectile |
| `src/gameplay/player/dash.rs` | 修改 — DashTrail, ExtendedInvuln |
| `src/gameplay/combat/hitbox.rs` | 修改 — LifestealSlash, Piercing |
| `src/gameplay/combat/projectiles.rs` | 修改 — PierceCount 组件, spawn 时挂载 |
| `src/gameplay/enemy/systems.rs` | 修改 — GoldBonus |
| `src/gameplay/progression/experience.rs` | 修改 — XpBonus |

**验证**：`cargo check --quiet` + `cargo test --quiet`

**状态**：✅ 已完成（Codex 实现，cargo check + cargo test 通过）

---

### 阶段 3b：Elite + Legendary 强化战斗效果实现（18 个）

**目标**：实现全部 10 个 Elite 和 8 个 Legendary 强化的战斗效果。

**实现策略**：
- 复杂效果（Whirlwind, Scatter, BulletStorm, Blink 等）需要修改攻击/冲刺的核心行为
- 简单数值效果（ArmorBreak, CritEnhance, KillHeal 等）在现有系统中添加条件分支
- 新增独立系统处理 Thorns（受伤反弹）、Phoenix（死亡复活）、Freeze（冻结）等
- 所有效果都通过 `Option<&AugmentInventory>` 查询，不影响无强化时的行为

---

#### Elite 强化（10 个）

**1. Whirlwind — 旋风斩**
- **效果**：近战攻击变为 360° 旋风（伤害 70%，升级: 100%）
- **文件**：`src/gameplay/player/combat.rs` → `spawn_player_melee_hitbox_with_mods`
- **实现**：当玩家有 Whirlwind 时，将 `ArcHitbox` 的 `half_angle_rad` 设为 `PI`（360°），伤害乘以 0.70/1.00

**2. ArmorBreak — 破甲**
- **效果**：近战命中降低敌人受伤抗性 20%（升级: 30%），持续 3s（升级: 5s）
- **文件**：`src/gameplay/augment/effects.rs` + `src/gameplay/combat/hitbox.rs`
- **新增组件**：`ArmorBroken { multiplier: f32, timer: Timer }` 在 effects.rs
- **实现**：
  1. 在 hitbox.rs 的 PlayerMelee 命中后，如果玩家有 ArmorBreak，给目标敌人挂 `ArmorBroken` 组件
  2. 在 effects.rs 新增 `tick_armor_break` 系统，倒计时并移除过期的 ArmorBroken
  3. 在 damage.rs 的 `apply_damage_events` 中，如果目标有 ArmorBroken，伤害 *= (1.0 + multiplier)

**3. Reflect — 弹幕反弹**
- **效果**：近战攻击反弹附近弹幕（升级: 反弹弹幕伤害 +50%）
- **文件**：`src/gameplay/augment/effects.rs`
- **实现**：新增 `melee_reflect_system`
  1. 检测玩家近战攻击时（通过 AnimationState::Attack 或 melee hitbox 存在）
  2. 查找附近的敌方弹丸（Team::Enemy 的 Projectile）
  3. 反转弹丸方向（velocity *= -1），改 team 为 Player
  4. 如果 stacks >= 2，damage *= 1.50

**4. Homing — 追踪弹**
- **效果**：弹丸轻微追踪最近敌人（升级: 强追踪）
- **文件**：`src/gameplay/augment/effects.rs`
- **新增组件**：`HomingProjectile { strength: f32 }` 
- **实现**：
  1. 在 combat.rs spawn_projectile 时，如果玩家有 Homing，挂载 HomingProjectile
  2. 新增 `homing_projectile_system`：每帧找最近敌人，微调弹丸 velocity 方向
  3. strength = if stacks >= 2 { 4.0 } else { 1.5 }（弧度/秒的转向速度）

**5. ChainLightning — 连锁闪电**
- **效果**：远程命中后闪电跳到 1 个附近敌人（50% 伤害，升级: 跳 2 个）
- **文件**：`src/gameplay/augment/effects.rs`
- **实现**：新增 `chain_lightning_system`
  1. 监听 `DamageAppliedEvent`，过滤 `DamageKind::PlayerRanged`
  2. 如果玩家有 ChainLightning，找目标附近最近的 1-2 个敌人（排除已命中的）
  3. 对每个链接目标发送新的 `DamageEvent`（damage * 0.50, kind = Passive）
  4. 跳数 = if stacks >= 2 { 2 } else { 1 }

**6. Scatter — 散射**
- **效果**：射击变为 3 发扇形（每发 50% 伤害，升级: 5 发）
- **文件**：`src/gameplay/player/combat.rs` → `spawn_ranged_burst`
- **实现**：当玩家有 Scatter 时，替换正常射击为扇形模式
  1. 弹丸数 = if stacks >= 2 { 5 } else { 3 }
  2. 扇形角度 = 30°（±15°）
  3. 每发伤害 = base_damage * 0.50
  4. 与 ExtraProjectile 互斥（Scatter 优先）

**7. Thorns — 荆棘**
- **效果**：受伤时反弹 15 点伤害（升级: 25 点）
- **文件**：`src/gameplay/augment/effects.rs`
- **实现**：新增 `thorns_system`
  1. 监听 `DamageAppliedEvent`，过滤目标为 Player
  2. 如果玩家有 Thorns，对 source 发送 `DamageEvent`（damage = 15/25, kind = Passive, team = Player）

**8. KillHeal — 击杀回血**
- **效果**：击杀回复 5 HP（升级: 8 HP）
- **文件**：`src/gameplay/enemy/systems.rs` → `enemy_death_system`
- **实现**：在敌人死亡时，如果玩家有 KillHeal：
  ```
  let heal = match stacks { 2 => 8.0, 1 => 5.0, _ => 0.0 };
  hp.current = (hp.current + heal).min(hp.max);
  ```
  注意：与现有 `lifesteal_on_kill` 奖励叠加

**9. CritEnhance — 暴击强化**
- **效果**：暴击率 +10%，暴击伤害 +30%（升级: +15%，+50%）
- **文件**：`src/gameplay/augment/effects.rs`
- **实现**：新增 `apply_crit_enhance_system`（OnEnter(InGame) 或在 AugmentSelect exit 时）
  - 简化方案：在 hitbox.rs 的命中检测中，如果玩家有 CritEnhance：
    - crit_chance += 0.10/0.15
    - crit_multiplier += 0.30/0.50
  - 在 `spawn_player_melee_hitbox_with_mods` 和 ranged spawn 中修改 Hitbox 的 crit 字段

**10. DashReset — 冲刺重置**
- **效果**：击杀敌人刷新冲刺冷却（升级: 击杀 +30% 移速 2s）
- **文件**：`src/gameplay/enemy/systems.rs` → `enemy_death_system`
- **新增组件**：`SpeedBuff { multiplier: f32, timer: Timer }` 在 effects.rs
- **实现**：
  1. 在 enemy_death_system 中，如果玩家有 DashReset，重置 DashCooldown timer
  2. 如果 stacks >= 2，额外挂载 SpeedBuff 组件
  3. 新增 `tick_speed_buff` 系统处理临时移速加成

---

#### Legendary 强化（8 个）

**11. SwordWave — 剑气**
- **效果**：近战释放远程剑气（35% 伤害，升级: 穿透 + 50% 伤害）
- **文件**：`src/gameplay/player/combat.rs` → `player_attack_input_system`
- **实现**：在近战攻击后，额外 spawn 一个 Projectile（team Player, kind PlayerMelee）
  1. 方向 = facing_direction，速度 = 500
  2. damage = attack_power * 0.35 / 0.50
  3. 如果 stacks >= 2，挂载 PierceCount { remaining: 255 }（无限穿透）

**12. Executioner — 处刑者**
- **效果**：敌人 HP<15% 时近战秒杀（升级: HP<25%）
- **文件**：`src/gameplay/combat/hitbox.rs` → `detect_hitbox_hurtbox_overlap`
- **实现**：在 PlayerMelee 命中后，检查目标 Health：
  ```
  let threshold = if stacks >= 2 { 0.25 } else { 0.15 };
  if target_health.current / target_health.max < threshold {
      damage = target_health.current + 1.0; // 秒杀
  }
  ```

**13. BulletStorm — 弹幕风暴**
- **效果**：终结技改为全屏弹幕（8 方向 ×3 波，升级: 12 方向 ×5 波）
- **文件**：`src/gameplay/player/systems.rs` → 终结技释放逻辑
- **实现**：当释放终结技时，如果有 BulletStorm，替换为弹幕模式
  1. 方向数 = if stacks >= 2 { 12 } else { 8 }
  2. 波数 = if stacks >= 2 { 5 } else { 3 }
  3. 每波间隔 0.15s，每发 damage = attack_power * 0.30

**14. Freeze — 冻结**
- **效果**：远程命中 15% 概率冻结敌人 1.5s（升级: 25% 概率，2s）
- **文件**：`src/gameplay/augment/effects.rs`
- **新增组件**：`Frozen { timer: Timer }` 
- **实现**：
  1. 在 DamageAppliedEvent 处理中（chain_lightning_system 同文件），检查 PlayerRanged 命中
  2. 随机判定冻结概率，成功则给敌人挂 Frozen 组件
  3. 新增 `tick_frozen_system`：冻结的敌人 MoveSpeed 设为 0，timer 到期后恢复
  4. 视觉：冻结时 Sprite color 变蓝

**15. DashShield — 冲刺护盾**
- **效果**：冲刺结束获得护盾（吸收 1 次伤害，3s，升级: 5s）
- **文件**：`src/gameplay/augment/effects.rs` + `src/gameplay/player/dash.rs`
- **新增组件**：`DashShieldBuff { timer: Timer }`
- **实现**：
  1. 在 dash.rs 冲刺结束时（dash.active 从 true 变 false），如果有 DashShield，挂载 DashShieldBuff
  2. 在 damage.rs apply_damage_events 中，如果玩家有 DashShieldBuff，吸收伤害并移除组件
  3. 新增 `tick_dash_shield` 系统处理超时移除

**16. Blink — 瞬移**
- **效果**：冲刺改为瞬移（无中间帧，升级: 距离 +50%）
- **文件**：`src/gameplay/player/dash.rs` → `player_dash_input_system` + `update_dash_state`
- **实现**：当有 Blink 时：
  1. 冲刺不走中间帧，直接 teleport 到目标位置
  2. 目标位置 = current_pos + dir * dash_distance * (if stacks >= 2 { 1.50 } else { 1.0 })
  3. dash_distance = dash.speed * dash.base_duration_s
  4. 设置 dash.active = false（跳过中间帧），但仍触发无敌

**17. Phoenix — 凤凰**
- **效果**：死亡时复活（50% HP，每局 1 次，升级: 80% HP）
- **文件**：`src/gameplay/augment/effects.rs`
- **新增组件**：`PhoenixUsed`（标记已使用）
- **实现**：新增 `phoenix_system`
  1. 检测玩家 Health.current <= 0 且有 Phoenix 且没有 PhoenixUsed
  2. 恢复 HP = max * (if stacks >= 2 { 0.80 } else { 0.50 })
  3. 挂载 PhoenixUsed 标记
  4. 触发屏幕闪光效果

**18. Greed — 贪婪**
- **效果**：每 100 金币 → +5% 伤害（升级: 每 80 金币）
- **文件**：`src/gameplay/player/combat.rs` → melee/ranged damage 计算处
- **实现**：在计算 damage 时查询 Gold 和 AugmentInventory：
  ```
  let greed_bonus = if has Greed {
      let threshold = if stacks >= 2 { 80 } else { 100 };
      (gold.0 / threshold) as f32 * 0.05
  } else { 0.0 };
  damage *= 1.0 + greed_bonus;
  ```

---

**影响文件**：
| 文件 | 操作 |
|------|------|
| `src/gameplay/augment/effects.rs` | 修改 — 新增 ArmorBroken, Frozen, DashShieldBuff, PhoenixUsed, SpeedBuff, HomingProjectile 组件 + 7 个新系统 |
| `src/gameplay/augment/mod.rs` | 修改 — 注册所有新系统 |
| `src/gameplay/player/combat.rs` | 修改 — Whirlwind, SwordWave, Scatter, CritEnhance, Greed |
| `src/gameplay/player/dash.rs` | 修改 — Blink, DashShield |
| `src/gameplay/combat/hitbox.rs` | 修改 — ArmorBreak 挂载, Executioner 秒杀 |
| `src/gameplay/combat/damage.rs` | 修改 — ArmorBroken 伤害加成, DashShield 吸收 |
| `src/gameplay/enemy/systems.rs` | 修改 — KillHeal, DashReset |
| `src/gameplay/player/systems.rs` | 修改 — BulletStorm 终结技替换 |

**验证**：`cargo check --quiet` + `cargo test --quiet`

---

**新建文件**：
- `src/gameplay/augment/mod.rs` — AugmentPlugin
- `src/gameplay/augment/data.rs` — AugmentId, AugmentRarity, AugmentCategory, AugmentDef, HeldAugment, AugmentInventory Component
- `src/gameplay/progression/mod.rs` — ProgressionPlugin
- `src/gameplay/progression/experience.rs` — PlayerLevel Component, XpGainEvent, LevelUpEvent, process_xp_gains system
- `assets/configs/augments.ron` — 30 个强化定义（id, category, rarity, title, description, upgraded_description, shop_cost）

**修改文件**：
- `src/gameplay/player/systems.rs` — spawn_player: 挂载 `AugmentInventory::default()` + `PlayerLevel::default()`，移除 `RuneLoadout`
- `src/gameplay/enemy/systems.rs` — enemy_death_system: 击杀时发送 `XpGainEvent`（普通怪 8-15 XP，精英 25-40 XP）
- `src/gameplay/enemy/boss.rs` — Boss 击杀发送大量 XP（100-200）
- `src/data/definitions.rs` — 新增 `AugmentConfig`/`AugmentsConfig` 结构，替换 `RuneConfig`/`RunesConfig`
- `src/data/loaders.rs` — 加载 `augments.ron` 替换 `runes.ron`
- `src/app.rs` — 注册 AugmentPlugin + ProgressionPlugin，移除旧 RunePlugin

**移除/替换**：
- `src/gameplay/rune/` 目录 → 被 `src/gameplay/augment/` 替代
- `RewardModifiers` 中的 `melee_mastery_stacks`/`ranged_mastery_stacks` 字段暂时保留（Phase 3 实现效果时再迁移）

**验证**：`cargo check` + `cargo test`

---

### 阶段 2：强化获取流程 + 升级选择 UI

**目标**：玩家能通过战斗、升级、Boss 获得强化和属性提升。

**新建文件**：
- `src/ui/augment_select.rs` — 强化选择 UI（展示 2-3 张强化卡片，玩家选 1 个）
- `src/ui/levelup_select.rs` — 升级属性选择 UI（展示 2-3 个属性选项，选 1 个）

**修改文件**：
- `src/states.rs` — AppState 新增 `AugmentSelect` 和 `LevelUpSelect` 状态
- `src/gameplay/session_core/mod.rs` — 重写 `on_room_cleared`：
  - 战斗房通关：40% 概率进入 AugmentSelect（普通池）
  - Boss 房通关：必定进入 AugmentSelect（精英/传说池）+ 自动属性大幅提升
  - 升级时进入 LevelUpSelect
- `src/gameplay/rewards/systems.rs` — 重写奖励流程，区分属性选择和强化选择
- `src/ui/reward_select.rs` — 简化，只保留 Boss 后的双重奖励模式；普通房不再用三选一
- `src/gameplay/session_core/mod.rs` — `generate_augment_choices`: 从 AugmentsConfig 按稀有度随机抽取 2-3 个
- `src/gameplay/progression/experience.rs` — LevelUpEvent 触发 AppState::LevelUpSelect

**关键逻辑**：
```
战斗房通关 → 金币 + XP → 40% 概率 AugmentSelect(Common)
精英房通关 → 金币 + XP → 必定 AugmentSelect(Common+Elite)
Boss 通关 → 金币 + 大量 XP + 自动属性提升 → 必定 AugmentSelect(Elite+Legendary)
XP 累积升级 → LevelUpSelect（2-3 属性选 1）
```

**验证**：`cargo check` + `cargo test` + 手动验证强化选择流程

---

### 阶段 3：强化战斗效果实现

**目标**：所有 30 个强化产生实际战斗效果。这是最大的阶段。

**修改文件**：
- `src/gameplay/player/combat.rs` — 近战系统读取 AugmentInventory，应用近战类强化效果
  - LifestealSlash: 命中后回血
  - HeavyStrike: 增加击退和伤害
  - ComboAccelerate: 连击加速
  - Whirlwind: 改变攻击为 360°
  - ArmorBreak: 命中标记敌人（新 Component `ArmorBroken`）
  - Reflect: 近战范围内反弹弹幕
  - SwordWave: 近战时生成远程剑气
  - Executioner: 低血秒杀判定
- `src/gameplay/player/systems.rs` — 远程系统读取 AugmentInventory
  - Piercing/SpeedBoost/ExtraProjectile: 修改弹丸属性
  - Homing: 弹丸追踪
  - ChainLightning: 命中后生成闪电跳
  - Scatter: 改变射击模式
  - BulletStorm: 替换终结技效果
  - Freeze: 命中概率冻结
- `src/gameplay/player/systems.rs` — 冲刺系统读取 AugmentInventory
  - DashTrail/DashEnergy/ExtendedInvuln/DashReset/DashShield/Blink
- `src/gameplay/combat/damage.rs` — 通用类强化
  - Thorns: 受伤反弹
  - KillHeal: 击杀回血
  - CritEnhance: 暴击修正
  - Phoenix: 死亡复活
  - Greed: 金币转伤害
- `src/gameplay/rewards/apply.rs` — GoldBonus/XpBonus/PickupRange 修正

**移除**：
- `RewardModifiers` 中的 `melee_mastery_stacks`/`ranged_mastery_stacks` 及相关逻辑
- 旧的 `EnhanceMeleeWeapon`/`EnhanceRangedWeapon` RewardType

**建议分批实现**：先做 10 个最常见的普通强化，再做精英，最后传说。每批验证编译。

**验证**：`cargo check` + `cargo test` + 手动验证各强化效果

---

### 阶段 4：事件房 + 商店扩展 + 祝福祠堂改造

**目标**：丰富非战斗房间内容，增加强化获取渠道。

**新建文件**：
- `src/gameplay/event_room/mod.rs` — EventRoomPlugin
- `src/gameplay/event_room/events.rs` — EventType 枚举（8 种事件）、事件触发和结算逻辑
- `src/ui/event_room.rs` — 事件房 UI（事件描述 + 选择按钮）

**修改文件**：
- `src/states.rs` — AppState 新增 `EventRoom` 状态
- `src/gameplay/map/generator.rs` — 调整房间类型权重：
  - 每层保证至少 1 个非战斗房（事件/商店）
  - 每层保证 1 个精英战斗房
  - `RoomType` 枚举新增 `Event` 和 `EliteCombat`
- `src/gameplay/session_core/mod.rs` — 
  - `on_room_enter`: Event 房间进入时触发随机事件
  - EliteCombat 房间通关必给强化选择
  - 祝福祠堂改造：提供传说强化 + 诅咒（复用诅咒系统）
- `src/gameplay/shop/mod.rs` — 商店扩展：
  - 新增强化商品区（2-3 个随机强化，价格：普通 40-60，精英 80-120，传说 150-200）
  - 新增消耗品（回血药水 30 金，临时攻击增益 50 金）
  - 新增诅咒移除服务（80 金）
- `src/ui/shop.rs` — 商店 UI 扩展显示新商品区

**验证**：`cargo check` + `cargo test` + 手动验证事件房和商店流程

---

### 阶段 5：新怪物 + 精英词缀 + TideHunter 调整

**目标**：扩充小怪池，精英差异化，Boss 威胁提升。

**修改文件**：
- `src/gameplay/enemy/components.rs` —
  - EnemyType 新增 `Bomber`, `Shielder`, `Summoner`
  - 新增 `EliteAffix` 枚举（Swift/Splitting/Shielded/Vampiric/Berserk/Teleporting）
  - 新增 `EliteAffixState` Component
  - Shielder 新增 `ShielderFacing` Component（朝向判定）
  - Bomber 新增 `BomberState` Component（蓄力计时器）
  - Summoner 新增 `SummonerState` Component（召唤冷却）
- `src/gameplay/enemy/systems.rs` —
  - `choose_enemy_types`: Floor 2+ 加入 Bomber，Floor 3+ 加入 Shielder，Floor 4+ 加入 Summoner
  - `spawn_enemy`: 精英生成时随机分配 1 个词缀，体型 1.3x，金色光环
  - 新增 Bomber AI：靠近 → 蓄力 → 自爆/被击杀
  - 新增 Shielder AI：正面朝向玩家推进，正面免疫远程
  - 新增 Summoner AI：远离玩家，周期召唤
- `src/gameplay/enemy/ai.rs` — 新增词缀行为系统：
  - `elite_affix_system`: Swift 加速、Berserk 狂暴检测、Teleporting 闪现
  - `elite_splitting_death`: Splitting 死亡分裂
  - `elite_shielded_system`: Shielded 护盾吸收
  - `elite_vampiric_system`: Vampiric 命中回血
- `assets/configs/enemies.ron` — 新增 Bomber/Shielder/Summoner 数值
- `src/gameplay/enemy/boss.rs` — TideHunter 调整：
  - Stalk 时间：1.2/0.8/0.5
  - 影子伤害：contact_damage × 1.0
  - P3 影子持续：6.0s
  - 穿越时对路径上玩家造成直接伤害
  - P2+ 目标预判玩家移动方向

**验证**：`cargo check` + `cargo test` + 手动验证新怪物和精英词缀

---

### 阶段 6：HUD + 平衡 + 收尾

**目标**：UI 完善，数值平衡，清理旧代码。

**修改文件**：
- `src/ui/hud.rs` —
  - 替换铭文槽位显示为强化图标列表（稀有度着色）
  - 新增 XP 条和等级显示
  - 精英词缀图标显示在敌人头顶
- `src/ui/pause.rs` — 暂停菜单显示完整强化列表和等级信息
- `assets/configs/game_balance.ron` — XP 曲线、强化掉落率、商店价格调整
- 清理旧代码：
  - 删除 `src/gameplay/rune/` 目录
  - 删除 `assets/configs/runes.ron`
  - 清理 `RewardModifiers` 中已迁移的精通字段
  - 清理 `RewardType` 中已废弃的枚举值

**验证**：`cargo check` + `cargo test` + 完整游玩测试 4 层

---

## 影响文件总览

| 文件 | 阶段 | 操作 |
|------|------|------|
| `src/gameplay/augment/mod.rs` | 1 | 新建 |
| `src/gameplay/augment/data.rs` | 1 | 新建 |
| `src/gameplay/progression/mod.rs` | 1 | 新建 |
| `src/gameplay/progression/experience.rs` | 1 | 新建 |
| `assets/configs/augments.ron` | 1 | 新建 |
| `src/ui/augment_select.rs` | 2 | 新建 |
| `src/ui/levelup_select.rs` | 2 | 新建 |
| `src/gameplay/event_room/mod.rs` | 4 | 新建 |
| `src/gameplay/event_room/events.rs` | 4 | 新建 |
| `src/ui/event_room.rs` | 4 | 新建 |
| `src/app.rs` | 1 | 修改 |
| `src/states.rs` | 2,4 | 修改 |
| `src/data/definitions.rs` | 1 | 修改 |
| `src/data/loaders.rs` | 1 | 修改 |
| `src/gameplay/player/systems.rs` | 1,3 | 修改 |
| `src/gameplay/player/combat.rs` | 3 | 修改 |
| `src/gameplay/enemy/systems.rs` | 1,5 | 修改 |
| `src/gameplay/enemy/components.rs` | 5 | 修改 |
| `src/gameplay/enemy/boss.rs` | 1,5 | 修改 |
| `src/gameplay/enemy/ai.rs` | 5 | 修改 |
| `src/gameplay/session_core/mod.rs` | 2,4 | 修改 |
| `src/gameplay/rewards/systems.rs` | 2 | 修改 |
| `src/gameplay/rewards/apply.rs` | 3 | 修改 |
| `src/gameplay/combat/damage.rs` | 3 | 修改 |
| `src/gameplay/shop/mod.rs` | 4 | 修改 |
| `src/gameplay/map/generator.rs` | 4 | 修改 |
| `src/ui/reward_select.rs` | 2 | 修改 |
| `src/ui/hud.rs` | 6 | 修改 |
| `src/ui/pause.rs` | 6 | 修改 |
| `assets/configs/enemies.ron` | 5 | 修改 |
| `assets/configs/game_balance.ron` | 6 | 修改 |
| `src/gameplay/rune/` | 6 | 删除 |
| `assets/configs/runes.ron` | 6 | 删除 |

---

## 验证方法

每个阶段完成后：
```bash
cargo check --quiet
cargo test --quiet
```

最终验证：手动游玩完整 4 层，检查：
1. 经验升级正常触发，属性选择 UI 正常
2. 强化在战斗房/精英房/Boss/商店/事件房正常获取
3. 30 个强化效果全部生效
4. 新怪物（Bomber/Shielder/Summoner）正常出现和行为
5. 精英词缀可见且有实际效果
6. TideHunter 威胁度明显提升
7. 事件房 8 种事件正常触发
8. 商店新增区域正常购买
