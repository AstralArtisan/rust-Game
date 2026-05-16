# 奖励系统重构 — 双轨奖励 + 铭文 + 事件房 + 祝福祠堂

## Context

当前奖励系统只有一个维度：属性加成（+ATK/+HP/+暴击等 10 种）。所有获取途径（清房/Boss/奖励房/商店）给的都是同质化的数值提升，缺乏构建感和策略深度。谜题房只有 3 种简单谜题，无风险无代价。奖励房进入即白拿，出现频率高。

参考 Hades（Boon + 锤子 + 混沌之门）、Slay the Spire（卡牌 + 遗物 + 问号房）、Isaac（道具 + 恶魔房）的设计，引入双轨成长系统：属性成长（安全稳定）+ 铭文系统（改变玩法），并重设计事件房和祝福祠堂。

本次为设计文档，实施将分多步迭代。第一步聚焦铭文数据模型 + 诅咒系统 + 祝福祠堂流程 + UI 基础。

---

## 一、双轨奖励系统总览

| 轨道 | 内容 | 获取方式 | 设计目标 |
|------|------|---------|---------|
| A：属性成长 | 现有 10 种属性加成 | 清房 3选1、Boss 双选、商店 | 稳定变强，面包黄油 |
| B：铭文系统 | 改造能力行为的被动效果 | 祝福祠堂、Boss、事件房、商店、清房低概率 | 改变玩法，构建差异化 |

两条轨道独立运作：属性让你"更强"，铭文让你"不同"。

---

## 二、铭文系统

### 2.1 铭文槽位

4 个槽位对应 4 种能力，每槽只能装 1 个铭文，装新的替换旧的：

| 槽位 | 对应能力 | 影响范围 |
|------|---------|---------|
| Melee | 近战攻击 | 近战命中效果、范围、攻速 |
| Ranged | 远程射击 | 弹道行为、射速、命中效果 |
| Dash | 冲刺 | 冲刺距离、无敌帧、附加效果 |
| Finisher | 当前装备的终结技 | 终结技行为改造 |

### 2.2 铭文等级

| 等级 | 获取途径 | 设计特点 |
|------|---------|---------|
| Common（普通） | 事件房、商店、清房低概率 | 微调型，轻微取舍或纯增益 |
| Elite（精英） | Boss、祝福祠堂、事件房、商店 | 改变打法，明显取舍 |
| Legendary（传说） | 祝福祠堂独占（附带诅咒） | 颠覆性效果 |

### 2.3 铭文内容池

#### 近战铭文

| ID | 名称 | 等级 | 效果 | 取舍 |
|----|------|------|------|------|
| ImpactWave | 命中冲击波 | Common | 近战命中释放小范围冲击波 | — |
| SlowOnHit | 霜击 | Common | 近战命中减速敌人 1s | — |
| ThirdStrikeExpand | 重击 | Common | 每第 3 下近战范围 ×1.5 | — |
| WhirlSlash | 回旋斩 | Elite | 近战变 360° 旋转攻击 | 攻速 -30% |
| ChainLightning | 连锁闪电 | Elite | 命中时闪电跳到附近 2 敌（40% 伤害） | — |
| ExplosiveFist | 爆裂拳 | Elite | 每第 3 次命中产生爆炸 | 前两下伤害 -15% |
| VampireBlade | 吸血刃 | Elite | 近战伤害 8% 转化为 HP | 近战范围 -25% |
| FrostTouch | 冰霜触碰 | Elite | 命中冻结敌人 0.5s | 攻击间隔 +20% |

#### 远程铭文

| ID | 名称 | 等级 | 效果 | 取舍 |
|----|------|------|------|------|
| PierceOne | 穿透弹 | Common | 弹道穿透 1 个敌人 | — |
| MarkOnHit | 标记弹 | Common | 命中标记敌人 3s（受伤 +15%） | — |
| RapidFireWeak | 速射 | Common | 射速 +30% | 伤害 -15% |
| Scatter | 散射弹 | Elite | 每次射击发射 3 颗弹 | 每颗伤害 -50% |
| HomingBullet | 追踪弹 | Elite | 弹道轻微追踪最近敌人 | 弹速 -30% |
| VenomShot | 毒液弹 | Elite | 命中附加 3s 持续伤害（60% 命中伤害） | 直接命中伤害 -20% |
| BarrageMode | 弹幕模式 | Elite | 射速 ×2 | 每颗伤害 -60% |

#### 冲刺铭文

| ID | 名称 | 等级 | 效果 | 取舍 |
|----|------|------|------|------|
| DashEndShockwave | 冲击波 | Common | 冲刺终点产生冲击波伤害周围敌人 | — |
| DashFirstCrit | 先手暴击 | Common | 冲刺后 1s 内首次攻击暴击率 +30% | — |
| Afterimage | 残影 | Common | 冲刺路径对经过的敌人造成伤害 | — |
| ShadowClone | 影分身 | Elite | 冲刺起点留下分身 2s，分身自动攻击 | — |
| PhaseDash | 相位冲刺 | Elite | 冲刺距离 ×1.5，全程无敌 | 冷却 +40% |
| BlinkDash | 闪现 | Elite | 冲刺变瞬移，冷却 -30% | 无无敌帧 |

#### 终结技铭文

| ID | 名称 | 等级 | 对应终结技 | 效果 |
|----|------|------|-----------|------|
| GroundSplitter | 裂地斩 | Elite | 剑气斩 | 剑气命中地面留下 3s 灼烧地带 |
| BoomerangBlade | 回旋刃 | Elite | 剑气斩 | 剑气飞出后返回，来回各一次伤害 |
| DeathChain | 死亡连锁 | Elite | 标记猎杀 | 被标记目标死亡时标记传递给最近敌人 |
| WeaknessExpose | 弱点暴露 | Elite | 标记猎杀 | 标记不造成即时伤害，目标 5s 受伤 ×2 |
| StormField | 雷暴领域 | Elite | 闪电冲刺 | 冲刺路径变为持续 4s 电场区域 |
| InstantThunder | 瞬雷 | Elite | 闪电冲刺 | 距离变 0，以自身为中心释放全屏闪电 |

#### 传说铭文（祝福祠堂独占，附带诅咒）

| ID | 名称 | 槽位 | 效果 |
|----|------|------|------|
| PhoenixSoul | 不死鸟 | Dash | 每层首次致死伤害改为回复 1HP + 2s 无敌 |
| Berserker | 狂战士 | Melee | HP < 30% 时攻击力 +50%、攻速 +30% |
| ThornBody | 荆棘之体 | Dash | 受伤时反弹 30% 伤害给攻击者 |
| EnergyShield | 能量护盾 | Finisher | 能量满时自动消耗 50 能量抵挡致命伤害 |

### 2.4 铭文协同示例（自由组合产生自然 build）

- 散射弹 + 毒液弹 = 3 颗毒弹，DOT 叠加
- 爆裂拳 + 连锁闪电 = 爆炸 + 闪电范围清场
- 影分身 + 弹幕模式 = 分身高速射击
- 狂战士 + 吸血刃 = 低血高伤 + 续航
- 先手暴击 + 回旋斩 = 冲刺后 360° 暴击清场
- 穿透弹 + 标记弹 = 一发标记整排敌人

---

## 三、诅咒系统

### 3.1 诅咒池

| ID | 名称 | 效果 | 持续房间数 |
|----|------|------|-----------|
| Fragile | 脆弱 | 受到伤害 +25% | 3 |
| Sluggish | 迟缓 | 移速 -20% | 3 |
| Exhaustion | 枯竭 | 能量获取 -40% | 3 |
| Exposed | 暴露 | 冲刺冷却 +50% | 2 |
| Weakness | 虚弱 | 造成伤害 -20% | 3 |

### 3.2 生命周期

1. 玩家在祝福祠堂选择铭文时，随机附带 1 个诅咒
2. 诅咒在选择时立即生效
3. 每次进入新房间时 `rooms_remaining -= 1`
4. `rooms_remaining == 0` 时自动消除，HUD 显示消除提示
5. **诅咒未消除时，不会再出现新的祝福房**（防止诅咒叠加失控）

### 3.3 数据模型

```rust
// src/gameplay/curse/mod.rs（新模块）
#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurseState {
    pub active: Vec<ActiveCurse>,
}

pub struct ActiveCurse {
    pub curse: CurseId,
    pub rooms_remaining: u32,
}
```

`CurseState` 作为 Component 挂在 Player entity 上。战斗系统在计算伤害/移速/能量/冷却时读取 `CurseState` 应用修正。

---

## 四、祝福祠堂（原 Reward 房重设计）

### 4.1 出现规则

- 每层最多 1 个祝福房
- 第 1 层不出现（玩家还在学习基础）
- 玩家身上有未消除的诅咒时，该层不再生成祝福房
- 房间类型保持 `RoomType::Reward`，但内部逻辑完全重写

### 4.2 流程

```
进入祝福祠堂
  → 展示 2 个铭文选项（精英/传说级）
  → 每个选项旁显示附带的随机诅咒
  → 玩家选择其一（铭文装入对应槽位 + 诅咒生效）
  → 或选择"离开"（不拿任何东西，房间不刷新）
  → 返回游戏
```

### 4.3 UI 设计

- 新增 `AppState::BlessingSelect`（或复用 `RewardSelect` 加新模式）
- 左右两列，每列：铭文名称 + 描述 + 取舍 + 对应诅咒
- 底部"离开"按钮
- 按 1/2 选择，Esc 离开

### 4.4 铭文选项生成

```
session_core::generate_blessing_choices(rng, floor, curse_pool) -> Vec<BlessingOffer>

BlessingOffer {
    rune: RuneId,
    curse: CurseId,
    curse_duration: u32,
}
```

- Floor 2-3：1 精英 + 1 精英，或 1 精英 + 1 传说
- Floor 4：必定 1 传说 + 1 精英
- 不会给出与玩家当前已装备铭文相同的选项

---

## 五、事件房（原谜题房重设计）

### 5.1 房间类型

`RoomType::Puzzle` 改名为 `RoomType::Event`（或保留 Puzzle 但扩展内容）。

事件房进入后随机触发以下类型之一：

### 5.2 事件类型

#### A. 挑战类（保留改进的谜题 + 新增）

| 事件 | 玩法 | 成功奖励 | 失败惩罚 |
|------|------|---------|---------|
| 陷阱生存（改进） | 存活 8s，陷阱更多更快 | 铭文选择（普通/精英） | 扣 15% 当前 HP |
| 限时歼灭 | 15s 内消灭一波精英怪 | 铭文选择（精英） | 无奖励 + 扣 10% HP |
| 无伤挑战 | 消灭一波普通怪，不能受伤 | 铭文选择（精英）+ 金币 | 无奖励（受伤即失败） |
| 压力板（改进） | 站板 2.5s，期间有弹幕需躲避 | 铭文选择（普通） | 扣 10% HP |

#### B. 随机事件类（Slay the Spire 问号房风格）

| 事件 | 描述 | 选项 |
|------|------|------|
| 神秘商人 | 一个可疑的商人出现 | A: 花 80 金获得随机精英铭文 / B: 花 30 金获得随机普通铭文 / C: 离开 |
| 血之祭坛 | 一个散发红光的祭坛 | A: 献祭 20% 当前 HP，获得精英铭文 / B: 献祭当前铭文，回复满血 / C: 离开 |
| 遗忘之泉 | 一池发光的泉水 | A: 饮用（随机效果：回满血 / 获得铭文 / 失去金币 / 获得诅咒） / B: 不喝 |
| 铁匠铺 | 一个沉默的铁匠 | A: 花 60 金升级当前铭文（普通→精英） / B: 花 30 金移除一个诅咒 / C: 离开 |

#### C. 赌博类

| 事件 | 描述 | 机制 |
|------|------|------|
| 命运转盘 | 一个旋转的轮盘 | 50% 获得精英铭文，30% 无事发生，20% 获得诅咒 |

### 5.3 事件房出现规则

- 每层 1-2 个事件房
- 事件类型随机，同一局不重复同一个随机事件
- 挑战类事件的难度随楼层增加

---

## 六、铭文获取途径汇总

| 来源 | 铭文等级 | 数量 | 条件 |
|------|---------|------|------|
| 祝福祠堂 | 精英/传说 | 2选1 | 附带诅咒 |
| Boss 击杀 | 精英 | 3选1 | 无代价，必定掉落 |
| 事件房（挑战） | 普通/精英 | 2选1 | 需完成挑战，失败有惩罚 |
| 事件房（随机事件） | 普通/精英 | 1 个 | 取决于事件选项 |
| 商店 | 普通/精英 | 1 个在售 | 花金币（普通 120-150，精英 200-280） |
| 清房（低概率） | 普通 | 混入属性 3选1 | ~15% 概率其中 1 个选项是铭文 |

**每局预期铭文获取：**
- Floor 1：0-1 个（事件房/商店/清房低概率）
- Floor 2：1-2 个（祝福房 + 事件房/商店）
- Floor 3：1-2 个（祝福房 + Boss + 事件房）
- Floor 4：1-2 个（Boss + 事件房/商店）
- 总计：约 4-7 个铭文机会，但只有 4 个槽位 → 需要做选择

---

## 七、商店扩展

### 7.1 商品池扩展

现有 8 种属性商品保留，新增铭文商品：

```
商店每次刷新：3 个属性商品 + 1 个铭文商品（如果有库存）
```

铭文商品价格：
- Common 铭文：基础价 120 + 楼层加成
- Elite 铭文：基础价 200 + 楼层加成

### 7.2 铭文商品生成

从未装备的铭文池中随机选择 1 个，优先选择与玩家当前 build 不冲突的槽位。

---

## 八、Boss 通关奖励调整

当前 Boss 通关给 DualBuff（2 个属性加成 ×1.5）。调整为：

```
Boss 通关 → 1 个属性加成（×1.5）+ 1 个铭文选择（3选1 精英级）
```

这让 Boss 击杀成为铭文的重要无代价来源，激励玩家挑战 Boss。

---

## 九、HUD 变更

### 9.1 铭文槽位显示

在 HUD 底部技能栏旁边（或上方），显示 4 个铭文槽位图标：
- 空槽：灰色边框
- 已装备：对应颜色图标 + 铭文名称缩写
- 悬停显示完整描述（可选，后续迭代）

### 9.2 诅咒状态显示

在 HP 条下方显示当前诅咒：
- 诅咒图标 + 剩余房间数
- 诅咒消除时短暂闪烁提示

---

## 十、影响文件

### 新增文件

| 文件 | 内容 |
|------|------|
| `src/gameplay/rune/mod.rs` | 铭文模块入口 |
| `src/gameplay/rune/data.rs` | RuneSlot, RuneTier, RuneId 枚举 |
| `src/gameplay/rune/apply.rs` | 铭文效果应用到战斗系统 |
| `src/gameplay/rune/systems.rs` | 铭文装备/替换事件处理 |
| `src/gameplay/curse/mod.rs` | 诅咒模块：CurseId, CurseState, 生命周期 |
| `src/gameplay/event_room/mod.rs` | 事件房逻辑（替代/扩展 puzzle/） |
| `assets/configs/runes.ron` | 铭文配置（名称、描述、取舍、价格） |
| `assets/configs/curses.ron` | 诅咒配置（名称、效果、持续时间） |
| `assets/configs/events.ron` | 事件房配置（事件类型、概率、奖惩） |

### 修改文件

| 文件 | 改动 |
|------|------|
| `src/gameplay/player/components.rs` | 新增 `RuneLoadout` 和 `CurseState` Component |
| `src/gameplay/session_core/mod.rs` | 祝福祠堂生成、Boss 奖励调整、铭文选项生成、事件房决策 |
| `src/gameplay/rewards/systems.rs` | RewardFlowMode 新增 Blessing 模式、铭文选择流程 |
| `src/gameplay/rewards/data.rs` | RewardOptionDraft 新增 Rune 变体 |
| `src/gameplay/shop/mod.rs` | ShopItem 新增铭文商品 |
| `src/gameplay/map/room.rs` | RoomType 可能新增 Event（或保留 Puzzle 扩展） |
| `src/gameplay/puzzle/mod.rs` | 扩展为事件房，新增事件类型 |
| `src/ui/reward_select.rs` | 祝福祠堂 UI、铭文选择 UI |
| `src/ui/shop.rs` | 铭文商品显示 |
| `src/ui/hud.rs` | 铭文槽位 + 诅咒状态显示 |
| `src/data/definitions.rs` | 新增 RunesConfig, CursesConfig, EventsConfig 解析 |
| `src/app.rs` | 注册新 Plugin（RunePlugin, CursePlugin, EventRoomPlugin） |
| `src/states.rs` | 可能新增 AppState::BlessingSelect |
| `assets/configs/rooms.ron` | 调整事件房/祝福房生成权重 |

---

## 十一、分步实施计划

### 第一步：铭文数据模型 + 诅咒系统 + 祝福祠堂（核心骨架）

1. 新建 `src/gameplay/rune/` 模块：RuneSlot, RuneTier, RuneId 枚举，RuneLoadout Component
2. 新建 `src/gameplay/curse/` 模块：CurseId, CurseState Component，房间计数递减系统
3. 新建 `assets/configs/runes.ron` 和 `assets/configs/curses.ron`
4. `data/definitions.rs` 新增配置解析
5. `player/components.rs` 给 Player 挂载 RuneLoadout + CurseState
6. `session_core/mod.rs` 新增 `generate_blessing_choices()` 函数
7. 祝福祠堂 UI（复用 reward_select 模式或新增 BlessingSelect 状态）
8. 祝福房出现规则（每层最多 1 个，有诅咒时不出现，Floor 1 不出现）
9. HUD 铭文槽位 + 诅咒状态显示

**此步不实现铭文的战斗效果**——只做数据流通：选择铭文 → 装入槽位 → HUD 显示。效果实现在第三步。

### 第二步：事件房 + 商店铭文 + Boss 奖励调整

1. 扩展谜题房为事件房：新增 2-3 种事件类型（限时歼灭、随机事件）
2. 事件房失败惩罚逻辑
3. 事件房奖励改为铭文选择
4. 商店新增铭文商品
5. Boss 通关奖励调整：1 属性 + 1 铭文
6. 清房 15% 概率混入铭文选项

### 第三步：铭文战斗效果实现（逐批迭代）

按优先级逐个实现铭文效果：
1. 近战 Common 3 个（最直观）
2. 远程 Common 3 个
3. 冲刺 Common 3 个
4. 近战 Elite 5 个
5. 远程 Elite 4 个
6. 冲刺 Elite 3 个
7. 终结技 Elite 6 个
8. 传说 4 个

每个铭文涉及对应战斗系统文件：
- 近战：`src/gameplay/player/combat.rs`
- 远程：`src/gameplay/combat/projectiles.rs`
- 冲刺：`src/gameplay/player/components.rs` DashState 相关
- 终结技：`src/gameplay/combat/damage.rs` 中的终结技执行逻辑

### 第四步：平衡与打磨

1. 铭文协同效果测试
2. 诅咒强度调整
3. 掉落概率微调
4. 事件房内容扩充

---

## 十二、验证方法

```bash
cargo check --quiet
cargo test --quiet
```

每步实施后手动验证：
- 第一步：进入祝福房 → 看到铭文选项 + 诅咒 → 选择后 HUD 显示铭文和诅咒倒计时 → 诅咒到期自动消除
- 第二步：事件房触发不同事件 → 商店出现铭文 → Boss 掉落铭文
- 第三步：装备铭文后战斗行为确实改变（如散射弹发射 3 颗）
