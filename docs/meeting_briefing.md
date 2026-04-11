# 组长例会汇报提纲

> 适用场景：第一次组长例会，刚结束开题报告，聚焦单机架构 + Rust 语言特性

---

## 1. 项目定位（一句话）

"勇闯方块城"——基于 Rust + Bevy 0.14 的 2D 俯视角 Roguelike，重点在玩法闭环、模块拆分、配置驱动和 Rust 语言特性的工程实践。

## 2. 技术选型

| 层面 | 选型 | 理由 |
|------|------|------|
| 语言 | Rust 2024 | 课程要求；所有权系统保证内存安全 |
| 框架 | Bevy 0.14 | 纯 Rust ECS 游戏引擎，Plugin 化组织 |
| 物理 | bevy_rapier2d 0.27 | 2D 碰撞检测，零重力俯视角 |
| 序列化 | serde + ron | 配置驱动 + 可读存档 |

## 3. 核心架构——五层分层 + 插件装配

```
┌─────────────────────────────────────────────────┐
│  启动/装配层    main.rs → GamePlugin (app.rs)   │
├─────────────────────────────────────────────────┤
│  基础设施层    core/ (资源、输入、音频、相机、   │
│               存档、成就、事件总线)               │
├─────────────────────────────────────────────────┤
│  数据定义层    data/ (RON配置 → GameDataRegistry)│
├─────────────────────────────────────────────────┤
│  玩法域层     gameplay/ (16个子模块)             │
│  player/ combat/ enemy/ map/ rewards/           │
│  progression/ session_core/ augment/ skills/... │
├─────────────────────────────────────────────────┤
│  表现层       ui/ (菜单、HUD、通知、结算)        │
│  网络层       coop/ pvp/ (联机，暂不展开)        │
└─────────────────────────────────────────────────┘
```

讲解要点：
- 整个游戏是一个 Bevy `App`，通过 `GamePlugin` 一次性装配 14 个一级插件
- `GameplayPlugin` 下再挂 12 个二级插件，每个负责独立的游戏子系统
- 状态机驱动：`AppState`（20 个状态）管理全局流程，`RoomState` 管理房间内推进
- 状态流转：`Loading → MainMenu → InGame ↔ RewardSelect/Shop/Paused → GameOver/Victory`

## 4. Rust 语言特性实际运用

### (a) ECS 模式 + Newtype 模式

```rust
// Component——纯数据，无行为
#[derive(Component)]
pub struct MoveSpeed(pub f32);   // newtype 防止参数混淆
pub struct AttackPower(pub f32); // 编译期捕获类型错误

// System——纯逻辑，通过 Query 借用 Component
fn player_move(q: Query<(&MoveSpeed, &mut Transform), With<Player>>) { ... }

// Resource——全局共享状态
#[derive(Resource)]
pub struct GameDataRegistry { pub player: PlayerConfig, pub enemies: EnemiesConfig, ... }
```

要点：数据与行为完全分离，组件是纯 struct，系统是纯函数。newtype 让 `MoveSpeed` 和 `AttackPower` 在类型层面不可混淆。

### (b) 枚举 + 穷尽匹配

```rust
// 20 个全局状态，编译器强制穷尽匹配
#[derive(States)]
pub enum AppState { Loading, MainMenu, InGame, Shop, GameOver, Victory, ... }

// 9 种敌人类型
pub enum EnemyType { MeleeChaser, RangedShooter, Charger, Flanker, Sniper, ... }

// 4 种 Boss
pub enum BossArchetype { Floor1Guardian, MirrorWarden, TideHunter, CubeCore }

// 6 种精英词缀
pub enum EliteAffix { Swift, Splitting, Shielded, Vampiric, Berserk, Teleporting }
```

要点：`match` 穷尽检查——新增敌人类型或游戏状态时，编译器会提醒所有未处理的分支，杜绝遗漏。

### (c) 所有权与借用——ParamSet 解决查询冲突

```rust
// 敌人 AI 需要先读取所有敌人位置（快照），再更新各自位置
// 两个 Query 可能借用同一实体的同一组件，Rust 不允许
fn enemy_ai(mut set: ParamSet<(
    Query<(&Transform, &EnemyStats), With<Enemy>>,  // 只读快照
    Query<&mut Transform, With<Enemy>>,              // 可写更新
)>) {
    // 第一阶段：读快照
    let snapshots: Vec<_> = set.p0().iter().collect();
    // 第二阶段：写更新
    for mut transform in set.p1().iter_mut() { ... }
}
```

要点：Rust 借用规则在 ECS 中的直接体现——不能同时持有不可变和可变引用。`ParamSet` 是 Bevy 提供的安全解法。

### (d) 生命周期标注

```rust
// session_core 中的规则效果持有多个可变引用
pub struct PlayerRuleEffects<'a> {
    pub health: &'a mut Health,
    pub energy: &'a mut Energy,
    pub move_speed: &'a mut MoveSpeed,
    pub attack_power: &'a mut AttackPower,
    pub crit: &'a mut CritChance,
    // ...
}
```

要点：显式生命周期 `'a` 让编译器验证这些可变引用不会悬垂，保证规则函数的内存安全。

### (e) Trait + derive 宏

```rust
// 一行 derive 自动实现序列化、ECS 注册、调试打印等能力
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Health { pub current: f32, pub max: f32 }

// Plugin trait 统一模块组织——每个子系统实现同一个接口
impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CombatSystemsPlugin);
    }
}
```

要点：derive 宏消除样板代码；Plugin trait 让 22 个子系统用统一接口注册到引擎。

### (f) 条件系统调度——单机/联机零拷贝复用

```rust
// 同一套增强系统，通过 .run_if() 条件组合
// 单机时在 InGame 运行，联机时只在 Host 端运行
.run_if(
    in_state(AppState::InGame).or_else(
        in_state(AppState::CoopGame)
            .and_then(is_coop_authority)
            .and_then(is_coop_simulation_active),
    ),
)
```

要点：不需要 if-else 分支复制代码，调度器在运行时根据条件决定是否执行系统。

### (g) 事件驱动解耦

```rust
// 定义事件
#[derive(Event)]
pub struct DamageEvent {
    pub target: Entity, pub source: Option<Entity>,
    pub amount: f32, pub kind: DamageKind, pub is_crit: bool, ...
}

// 生产者发送
damage_events.send(DamageEvent { ... });

// 消费者读取——完全不知道谁发的
for event in damage_events.read() { ... }
```

要点：11 种事件类型构成事件总线，战斗系统、音效系统、成就系统、UI 之间零耦合。

### (h) serde + RON 配置驱动

```rust
// 全部平衡参数从 assets/configs/*.ron 加载
// 11 个配置文件：player.ron, enemies.ron, boss.ron, rewards.ron, ...
// 加载失败自动回退 default_registry()，保证游戏可启动
```

要点：调数值改配置文件即可，不碰 Rust 源码。RON 格式可读性好，支持注释。

## 5. 当前进度

- 单机主循环闭环：主菜单 → 战斗 → 奖励/商店/事件房 → Boss → 下一层/结算
- 内容量：9 种敌人 AI、4 种 Boss、6 种精英词缀、30 种被动增强、5 种诅咒、4 种主动技能
- 配置驱动：11 个 RON 配置文件覆盖全部数值
- 测试覆盖：44 个单元测试，覆盖 XP 曲线、Boss 决策、奖励规则等核心逻辑
- 联机原型：Coop（Lightyear 主机权威）和 PVP（自定义 UDP）已跑通，后续重点完善

## 6. 后续计划

- 清理已废弃的铭文系统残留代码
- 修复插件注册位置不一致等架构问题（详见架构修改建议文档）
- 完善单机玩法深度（新敌人、新增强、数值平衡）
- 推进联机功能稳定化
